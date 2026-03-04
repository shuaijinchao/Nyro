use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::Value;

use crate::protocol::types::*;
use crate::protocol::EgressEncoder;

pub struct AnthropicEncoder;

impl EgressEncoder for AnthropicEncoder {
    fn encode_request(&self, req: &InternalRequest) -> Result<(Value, HeaderMap)> {
        let mut system_text = String::new();
        let mut messages = Vec::new();

        for msg in &req.messages {
            if msg.role == Role::System {
                if !system_text.is_empty() {
                    system_text.push('\n');
                }
                system_text.push_str(&msg.content.as_text());
                continue;
            }

            messages.push(encode_message(msg)?);
        }

        let max_tokens = req.max_tokens.unwrap_or(4096);

        let mut body = serde_json::json!({
            "model": req.model,
            "messages": messages,
            "max_tokens": max_tokens,
            "stream": req.stream,
        });

        let obj = body.as_object_mut().unwrap();

        if !system_text.is_empty() {
            obj.insert("system".into(), Value::String(system_text));
        }
        if let Some(t) = req.temperature {
            obj.insert("temperature".into(), t.into());
        }
        if let Some(p) = req.top_p {
            obj.insert("top_p".into(), p.into());
        }

        if let Some(ref tools) = req.tools {
            let tools_val: Vec<Value> = tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.parameters,
                    })
                })
                .collect();
            obj.insert("tools".into(), Value::Array(tools_val));
        }

        if let Some(ref tc) = req.tool_choice {
            obj.insert("tool_choice".into(), tc.clone());
        }

        let mut headers = HeaderMap::new();
        headers.insert(
            "anthropic-version",
            HeaderValue::from_static("2023-06-01"),
        );

        Ok((body, headers))
    }

    fn egress_path(&self, _model: &str, _stream: bool) -> String {
        "/v1/messages".to_string()
    }
}

fn encode_message(msg: &InternalMessage) -> Result<Value> {
    let role = match msg.role {
        Role::User | Role::Tool => "user",
        Role::Assistant => "assistant",
        Role::System => unreachable!("system handled separately"),
    };

    let content = match &msg.content {
        MessageContent::Text(t) => {
            if msg.tool_call_id.is_some() {
                Value::Array(vec![serde_json::json!({
                    "type": "tool_result",
                    "tool_use_id": msg.tool_call_id,
                    "content": t,
                })])
            } else if let Some(ref tcs) = msg.tool_calls {
                let mut blocks: Vec<Value> = vec![];
                if !t.is_empty() {
                    blocks.push(serde_json::json!({"type": "text", "text": t}));
                }
                for tc in tcs {
                    let input: Value =
                        serde_json::from_str(&tc.arguments).unwrap_or(Value::Object(Default::default()));
                    blocks.push(serde_json::json!({
                        "type": "tool_use",
                        "id": tc.id,
                        "name": tc.name,
                        "input": input,
                    }));
                }
                Value::Array(blocks)
            } else {
                Value::String(t.clone())
            }
        }
        MessageContent::Blocks(blocks) => {
            let arr: Vec<Value> = blocks
                .iter()
                .map(|b| match b {
                    ContentBlock::Text { text } => {
                        serde_json::json!({"type": "text", "text": text})
                    }
                    ContentBlock::Image { source } => {
                        serde_json::json!({
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": source.media_type,
                                "data": source.data,
                            }
                        })
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        serde_json::json!({
                            "type": "tool_use",
                            "id": id,
                            "name": name,
                            "input": input,
                        })
                    }
                    ContentBlock::ToolResult {
                        tool_use_id,
                        content,
                    } => {
                        serde_json::json!({
                            "type": "tool_result",
                            "tool_use_id": tool_use_id,
                            "content": content,
                        })
                    }
                })
                .collect();
            Value::Array(arr)
        }
    };

    Ok(serde_json::json!({
        "role": role,
        "content": content,
    }))
}
