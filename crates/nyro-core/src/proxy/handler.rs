use std::convert::Infallible;
use std::time::Instant;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures::StreamExt;
use serde_json::Value;
use tokio_stream::wrappers::ReceiverStream;

use crate::db::models::Provider;
use crate::logging::LogEntry;
use crate::protocol::gemini::decoder::GeminiDecoder;
use crate::protocol::types::*;
use crate::protocol::Protocol;
use crate::proxy::client::ProxyClient;
use crate::Gateway;

// ── OpenAI ingress: POST /v1/chat/completions ──

pub async fn openai_proxy(State(gw): State<Gateway>, Json(body): Json<Value>) -> Response {
    universal_proxy(gw, body, Protocol::OpenAI).await
}

// ── Anthropic ingress: POST /v1/messages ──

pub async fn anthropic_proxy(State(gw): State<Gateway>, Json(body): Json<Value>) -> Response {
    universal_proxy(gw, body, Protocol::Anthropic).await
}

// ── Gemini ingress: POST /v1beta/models/:model_action ──

pub async fn gemini_proxy(
    State(gw): State<Gateway>,
    Path(model_action): Path<String>,
    Json(body): Json<Value>,
) -> Response {
    let (model, action) = match model_action.rsplit_once(':') {
        Some((m, a)) => (m.to_string(), a.to_string()),
        None => (model_action.clone(), "generateContent".to_string()),
    };
    let is_stream = action == "streamGenerateContent";

    let decoder = GeminiDecoder;
    let internal = match decoder.decode_with_model(body, &model, is_stream) {
        Ok(r) => r,
        Err(e) => return error_response(400, &format!("invalid Gemini request: {e}")),
    };

    proxy_pipeline(gw, internal, Protocol::Gemini).await
}

// ── Universal proxy pipeline ──

async fn universal_proxy(gw: Gateway, body: Value, ingress: Protocol) -> Response {
    let decoder = crate::protocol::get_decoder(ingress);
    let internal = match decoder.decode_request(body) {
        Ok(r) => r,
        Err(e) => return error_response(400, &format!("invalid request: {e}")),
    };

    proxy_pipeline(gw, internal, ingress).await
}

async fn proxy_pipeline(gw: Gateway, internal: InternalRequest, ingress: Protocol) -> Response {
    let start = Instant::now();
    let request_model = internal.model.clone();
    let is_stream = internal.stream;

    let route = {
        let cache = gw.route_cache.read().await;
        cache.match_route(&request_model).cloned()
    };
    let route = match route {
        Some(r) => r,
        None => return error_response(404, &format!("no route for model: {request_model}")),
    };

    let provider = match get_provider(&gw, &route.target_provider).await {
        Ok(p) => p,
        Err(e) => return error_response(502, &format!("provider error: {e}")),
    };

    let egress: Protocol = provider.protocol.parse().unwrap_or(Protocol::OpenAI);

    let encoder = crate::protocol::get_encoder(egress);
    let (egress_body, extra_headers) = match encoder.encode_request(&internal) {
        Ok(r) => r,
        Err(e) => return error_response(500, &format!("encode error: {e}")),
    };

    let actual_model = if route.target_model.is_empty() || route.target_model == "*" {
        request_model.clone()
    } else {
        route.target_model.clone()
    };

    let egress_body = override_model(egress_body, &actual_model, egress);
    let egress_path = encoder.egress_path(&actual_model, is_stream);

    let client = ProxyClient::new(gw.http_client.clone());
    let ingress_str = ingress.to_string();
    let egress_str = egress.to_string();

    if is_stream {
        handle_stream(
            gw,
            client,
            &provider,
            egress,
            ingress,
            &egress_path,
            egress_body,
            extra_headers,
            &ingress_str,
            &egress_str,
            &request_model,
            &actual_model,
            start,
        )
        .await
    } else {
        handle_non_stream(
            gw,
            client,
            &provider,
            egress,
            ingress,
            &egress_path,
            egress_body,
            extra_headers,
            &ingress_str,
            &egress_str,
            &request_model,
            &actual_model,
            start,
        )
        .await
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_non_stream(
    gw: Gateway,
    client: ProxyClient,
    provider: &Provider,
    egress: Protocol,
    ingress: Protocol,
    path: &str,
    body: Value,
    extra_headers: reqwest::header::HeaderMap,
    ingress_str: &str,
    egress_str: &str,
    request_model: &str,
    actual_model: &str,
    start: Instant,
) -> Response {
    let (resp, status) = match client
        .call_non_stream(
            &provider.base_url,
            path,
            &provider.api_key,
            egress,
            body,
            extra_headers,
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            emit_log(
                &gw, ingress_str, egress_str, request_model, actual_model,
                &provider.name, 502, start.elapsed().as_millis() as f64,
                TokenUsage::default(), false, false,
                Some(e.to_string()), None, None,
            );
            return error_response(502, &format!("upstream error: {e}"));
        }
    };

    if status >= 400 {
        let preview = serde_json::to_string(&resp).ok().map(|s| s.chars().take(500).collect());
        emit_log(
            &gw, ingress_str, egress_str, request_model, actual_model,
            &provider.name, status as i32, start.elapsed().as_millis() as f64,
            TokenUsage::default(), false, false,
            preview.clone(), None, None,
        );
        return (
            StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY),
            Json(resp),
        )
            .into_response();
    }

    let parser = crate::protocol::get_response_parser(egress);
    let formatter = crate::protocol::get_response_formatter(ingress);

    let internal_resp = match parser.parse_response(resp) {
        Ok(r) => r,
        Err(e) => return error_response(500, &format!("parse error: {e}")),
    };

    let is_tool = !internal_resp.tool_calls.is_empty();
    let usage = internal_resp.usage.clone();
    let output = formatter.format_response(&internal_resp);

    let response_preview = serde_json::to_string(&output)
        .ok()
        .map(|s| s.chars().take(500).collect());

    emit_log(
        &gw, ingress_str, egress_str, request_model, actual_model,
        &provider.name, status as i32, start.elapsed().as_millis() as f64,
        usage, false, is_tool, None, None, response_preview,
    );

    (
        StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
        Json(output),
    )
        .into_response()
}

#[allow(clippy::too_many_arguments)]
async fn handle_stream(
    gw: Gateway,
    client: ProxyClient,
    provider: &Provider,
    egress: Protocol,
    ingress: Protocol,
    path: &str,
    body: Value,
    extra_headers: reqwest::header::HeaderMap,
    ingress_str: &str,
    egress_str: &str,
    request_model: &str,
    actual_model: &str,
    start: Instant,
) -> Response {
    let (resp, status) = match client
        .call_stream(
            &provider.base_url,
            path,
            &provider.api_key,
            egress,
            body,
            extra_headers,
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            emit_log(
                &gw, ingress_str, egress_str, request_model, actual_model,
                &provider.name, 502, start.elapsed().as_millis() as f64,
                TokenUsage::default(), true, false,
                Some(e.to_string()), None, None,
            );
            return error_response(502, &format!("upstream error: {e}"));
        }
    };

    if status >= 400 {
        let err_body: Value = resp
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({"error": {"message": "upstream error"}}));
        emit_log(
            &gw, ingress_str, egress_str, request_model, actual_model,
            &provider.name, status as i32, start.elapsed().as_millis() as f64,
            TokenUsage::default(), true, false,
            Some(err_body.to_string()), None, None,
        );
        return (
            StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY),
            Json(err_body),
        )
            .into_response();
    }

    let mut stream_parser = crate::protocol::get_stream_parser(egress);
    let mut stream_formatter = crate::protocol::get_stream_formatter(ingress);

    let mut byte_stream = resp.bytes_stream();
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<String, Infallible>>(64);

    let gw_log = gw.clone();
    let provider_name = provider.name.clone();
    let ingress_s = ingress_str.to_string();
    let egress_s = egress_str.to_string();
    let req_model = request_model.to_string();
    let act_model = actual_model.to_string();

    tokio::spawn(async move {
        while let Some(chunk) = byte_stream.next().await {
            let bytes = match chunk {
                Ok(b) => b,
                Err(_) => break,
            };
            let text = String::from_utf8_lossy(&bytes);
            if let Ok(deltas) = stream_parser.parse_chunk(&text) {
                let events = stream_formatter.format_deltas(&deltas);
                for ev in events {
                    if tx.send(Ok(ev.to_sse_string())).await.is_err() {
                        return;
                    }
                }
            }
        }

        if let Ok(deltas) = stream_parser.finish() {
            let events = stream_formatter.format_deltas(&deltas);
            for ev in events {
                let _ = tx.send(Ok(ev.to_sse_string())).await;
            }
        }

        let done_events = stream_formatter.format_done();
        for ev in done_events {
            let _ = tx.send(Ok(ev.to_sse_string())).await;
        }

        let usage = stream_formatter.usage();
        emit_log(
            &gw_log, &ingress_s, &egress_s, &req_model, &act_model,
            &provider_name, 200, start.elapsed().as_millis() as f64,
            usage, true, false, None, None, None,
        );
    });

    let stream = ReceiverStream::new(rx);
    let body = Body::from_stream(stream);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::CONNECTION, "keep-alive")
        .body(body)
        .unwrap()
}

// ── Helpers ──

async fn get_provider(gw: &Gateway, id: &str) -> anyhow::Result<Provider> {
    sqlx::query_as::<_, Provider>(
        "SELECT id, name, protocol, base_url, api_key, is_active, priority, created_at, updated_at \
         FROM providers WHERE id = ? AND is_active = 1",
    )
    .bind(id)
    .fetch_optional(&gw.db)
    .await?
    .ok_or_else(|| anyhow::anyhow!("provider not found or inactive: {id}"))
}

fn override_model(mut body: Value, model: &str, protocol: Protocol) -> Value {
    match protocol {
        Protocol::Gemini => body,
        _ => {
            if let Some(obj) = body.as_object_mut() {
                obj.insert("model".into(), Value::String(model.to_string()));
            }
            body
        }
    }
}

fn error_response(status: u16, message: &str) -> Response {
    let code = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (
        code,
        Json(serde_json::json!({
            "error": {
                "message": message,
                "type": "gateway_error",
                "code": status
            }
        })),
    )
        .into_response()
}

fn emit_log(
    gw: &Gateway,
    ingress: &str,
    egress: &str,
    request_model: &str,
    actual_model: &str,
    provider_name: &str,
    status_code: i32,
    duration_ms: f64,
    usage: TokenUsage,
    is_stream: bool,
    is_tool_call: bool,
    error_message: Option<String>,
    request_preview: Option<String>,
    response_preview: Option<String>,
) {
    let _ = gw.log_tx.try_send(LogEntry {
        ingress_protocol: ingress.to_string(),
        egress_protocol: egress.to_string(),
        request_model: request_model.to_string(),
        actual_model: actual_model.to_string(),
        provider_name: provider_name.to_string(),
        status_code,
        duration_ms,
        usage,
        is_stream,
        is_tool_call,
        error_message,
        request_preview,
        response_preview,
    });
}
