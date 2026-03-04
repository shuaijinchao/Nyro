use anyhow::Result;
use serde_json::Value;

use crate::protocol::types::*;
use crate::protocol::{IngressDecoder, Protocol};

use super::types::*;

pub struct AnthropicDecoder;

impl IngressDecoder for AnthropicDecoder {
    fn decode_request(&self, body: Value) -> Result<InternalRequest> {
        let req: AnthropicRequest = serde_json::from_value(body)?;

        let mut messages = Vec::new();

        if let Some(system) = &req.system {
            let text = match system {
                AnthropicSystem::Text(t) => t.clone(),
                AnthropicSystem::Blocks(blocks) => blocks
                    .iter()
                    .filter_map(|b| match b {
                        AnthropicContentBlock::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            };
            messages.push(InternalMessage {
                role: Role::System,
                content: MessageContent::Text(text),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        for msg in req.messages {
            messages.push(decode_message(msg)?);
        }

        let tools = req.tools.map(|tools| {
            tools
                .into_iter()
                .map(|t| ToolDef {
                    name: t.name,
                    description: t.description,
                    parameters: t.input_schema,
                })
                .collect()
        });

        Ok(InternalRequest {
            messages,
            model: req.model,
            stream: req.stream,
            temperature: req.temperature,
            max_tokens: Some(req.max_tokens),
            top_p: req.top_p,
            tools,
            tool_choice: req.tool_choice,
            source_protocol: Protocol::Anthropic,
            extra: Default::default(),
        })
    }
}

fn decode_message(msg: AnthropicMessage) -> Result<InternalMessage> {
    let role = match msg.role.as_str() {
        "user" => Role::User,
        "assistant" => Role::Assistant,
        other => anyhow::bail!("unknown Anthropic role: {other}"),
    };

    let (content, tool_calls, tool_call_id) = match msg.content {
        AnthropicContent::Text(t) => (MessageContent::Text(t), None, None),
        AnthropicContent::Blocks(blocks) => {
            let mut content_blocks = Vec::new();
            let mut tcs = Vec::new();
            let mut tc_id = None;

            for block in blocks {
                match block {
                    AnthropicContentBlock::Text { text } => {
                        content_blocks.push(ContentBlock::Text { text });
                    }
                    AnthropicContentBlock::Image { source } => {
                        content_blocks.push(ContentBlock::Image {
                            source: ImageSource {
                                media_type: source.media_type,
                                data: source.data,
                            },
                        });
                    }
                    AnthropicContentBlock::ToolUse { id, name, input } => {
                        tcs.push(ToolCall {
                            id: id.clone(),
                            name: name.clone(),
                            arguments: input.to_string(),
                        });
                        content_blocks.push(ContentBlock::ToolUse { id, name, input });
                    }
                    AnthropicContentBlock::ToolResult {
                        tool_use_id,
                        content,
                    } => {
                        tc_id = Some(tool_use_id.clone());
                        content_blocks.push(ContentBlock::ToolResult {
                            tool_use_id,
                            content: content.unwrap_or(Value::Null),
                        });
                    }
                }
            }

            let tool_calls_opt = if tcs.is_empty() { None } else { Some(tcs) };

            if content_blocks.len() == 1 {
                if let ContentBlock::Text { text } = &content_blocks[0] {
                    return Ok(InternalMessage {
                        role,
                        content: MessageContent::Text(text.clone()),
                        tool_calls: tool_calls_opt,
                        tool_call_id: tc_id,
                    });
                }
            }

            (
                MessageContent::Blocks(content_blocks),
                tool_calls_opt,
                tc_id,
            )
        }
    };

    Ok(InternalMessage {
        role,
        content,
        tool_calls,
        tool_call_id,
    })
}
