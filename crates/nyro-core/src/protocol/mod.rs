pub mod types;
pub mod openai;
pub mod anthropic;
pub mod gemini;

use reqwest::header::HeaderMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    OpenAI,
    Anthropic,
    Gemini,
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::OpenAI => write!(f, "openai"),
            Protocol::Anthropic => write!(f, "anthropic"),
            Protocol::Gemini => write!(f, "gemini"),
        }
    }
}

impl std::str::FromStr for Protocol {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(Protocol::OpenAI),
            "anthropic" => Ok(Protocol::Anthropic),
            "gemini" => Ok(Protocol::Gemini),
            _ => anyhow::bail!("unknown protocol: {s}"),
        }
    }
}

// ── Client → Gateway ──

pub trait IngressDecoder {
    fn decode_request(&self, body: serde_json::Value) -> anyhow::Result<types::InternalRequest>;
}

// ── Gateway → Provider ──

pub trait EgressEncoder {
    fn encode_request(
        &self,
        req: &types::InternalRequest,
    ) -> anyhow::Result<(serde_json::Value, HeaderMap)>;

    fn egress_path(&self, model: &str, stream: bool) -> String;
}

// ── Provider response → internal ──

pub trait ResponseParser: Send {
    fn parse_response(
        &self,
        resp: serde_json::Value,
    ) -> anyhow::Result<types::InternalResponse>;
}

// ── Internal → client response ──

pub trait ResponseFormatter: Send {
    fn format_response(&self, resp: &types::InternalResponse) -> serde_json::Value;
}

// ── Streaming: provider → internal deltas ──

pub trait StreamParser: Send {
    fn parse_chunk(&mut self, raw: &str) -> anyhow::Result<Vec<types::StreamDelta>>;
    fn finish(&mut self) -> anyhow::Result<Vec<types::StreamDelta>>;
}

// ── Streaming: internal deltas → client SSE ──

pub trait StreamFormatter: Send {
    fn format_deltas(&mut self, deltas: &[types::StreamDelta]) -> Vec<SseEvent>;
    fn format_done(&mut self) -> Vec<SseEvent>;
    fn usage(&self) -> types::TokenUsage;
}

// ── SSE helper ──

#[derive(Debug, Clone)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
}

impl SseEvent {
    pub fn new(event: Option<&str>, data: impl Into<String>) -> Self {
        Self {
            event: event.map(|e| e.to_string()),
            data: data.into(),
        }
    }

    pub fn to_sse_string(&self) -> String {
        let mut s = String::new();
        if let Some(ref event) = self.event {
            s.push_str(&format!("event: {event}\n"));
        }
        s.push_str(&format!("data: {}\n\n", self.data));
        s
    }
}

// ── Factory functions ──

pub fn get_decoder(protocol: Protocol) -> Box<dyn IngressDecoder + Send> {
    match protocol {
        Protocol::OpenAI => Box::new(openai::decoder::OpenAIDecoder),
        Protocol::Anthropic => Box::new(anthropic::decoder::AnthropicDecoder),
        Protocol::Gemini => Box::new(gemini::decoder::GeminiDecoder),
    }
}

pub fn get_encoder(protocol: Protocol) -> Box<dyn EgressEncoder + Send> {
    match protocol {
        Protocol::OpenAI => Box::new(openai::encoder::OpenAIEncoder),
        Protocol::Anthropic => Box::new(anthropic::encoder::AnthropicEncoder),
        Protocol::Gemini => Box::new(gemini::encoder::GeminiEncoder),
    }
}

pub fn get_response_parser(protocol: Protocol) -> Box<dyn ResponseParser> {
    match protocol {
        Protocol::OpenAI => Box::new(openai::stream::OpenAIResponseParser),
        Protocol::Anthropic => Box::new(anthropic::stream::AnthropicResponseParser),
        Protocol::Gemini => Box::new(gemini::stream::GeminiResponseParser),
    }
}

pub fn get_response_formatter(protocol: Protocol) -> Box<dyn ResponseFormatter> {
    match protocol {
        Protocol::OpenAI => Box::new(openai::stream::OpenAIResponseFormatter),
        Protocol::Anthropic => Box::new(anthropic::stream::AnthropicResponseFormatter),
        Protocol::Gemini => Box::new(gemini::stream::GeminiResponseFormatter),
    }
}

pub fn get_stream_parser(protocol: Protocol) -> Box<dyn StreamParser> {
    match protocol {
        Protocol::OpenAI => Box::new(openai::stream::OpenAIStreamParser::new()),
        Protocol::Anthropic => Box::new(anthropic::stream::AnthropicStreamParser::new()),
        Protocol::Gemini => Box::new(gemini::stream::GeminiStreamParser::new()),
    }
}

pub fn get_stream_formatter(protocol: Protocol) -> Box<dyn StreamFormatter> {
    match protocol {
        Protocol::OpenAI => Box::new(openai::stream::OpenAIStreamFormatter::new()),
        Protocol::Anthropic => Box::new(anthropic::stream::AnthropicStreamFormatter::new()),
        Protocol::Gemini => Box::new(gemini::stream::GeminiStreamFormatter::new()),
    }
}
