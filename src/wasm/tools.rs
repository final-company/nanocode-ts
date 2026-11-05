use std::sync::Arc;

use nanocode::{
    FunctionHandler, FunctionTool, ToolRecovery,
    tool_contract::{Tool, ToolContext, ToolInput, ToolOutput},
};
use nanocode_code_mode::embedded::HostFuture;
use serde::Deserialize;
use serde_json::{Value, json, value::RawValue};

use super::bridge::call;

#[derive(Deserialize)]
pub struct HostTool {
    name: String,
    description: String,
    parameters: Value,
    #[serde(default)]
    content: bool,
}

struct Bridge;

impl FunctionHandler for Bridge {
    fn call<'a>(
        &'a self,
        name: &'a str,
        input: Value,
        context: ToolContext<'a>,
    ) -> HostFuture<'a, Result<Value, String>> {
        Box::pin(async move {
            let context = context_value(context).await;
            call(json!({"op": "tool_call", "name": name, "input": input, "context": context}))
                .await
                .map_err(|error| {
                    error
                        .get("detail")
                        .and_then(Value::as_str)
                        .map_or_else(|| error.to_string(), str::to_owned)
                })
        })
    }
}

impl HostTool {
    pub fn name(&self) -> &str {
        &self.name
    }
}

pub fn tools(definitions: Vec<HostTool>) -> impl Iterator<Item = FunctionTool> {
    let bridge: Arc<dyn FunctionHandler> = Arc::new(Bridge);
    definitions.into_iter().map(move |tool| {
        FunctionTool::new(
            tool.name,
            tool.description,
            tool.parameters,
            Arc::clone(&bridge),
        )
        .parallel_tool_calls(true)
        .content_results(tool.content)
    })
}

async fn context_value(context: ToolContext<'_>) -> Value {
    json!({
        "session_id": context.session_id(),
        "call_id": context.call_id(),
        "turn_id": context.turn_id(),
        "model": context.model(),
        "instruction_revision": context.instruction_revision(),
        "origin": context.host_context(),
        "subagent": super::subagent(context.session_id()).await,
    })
}

pub(crate) struct Recovery {
    tools: Vec<(String, bool)>,
}

impl Recovery {
    pub(crate) fn new(tools: &[HostTool]) -> Self {
        Self {
            tools: tools
                .iter()
                .map(|tool| (tool.name.clone(), tool.content))
                .collect(),
        }
    }
}

struct RecoveredResult(Value);

impl FunctionHandler for RecoveredResult {
    fn call<'a>(
        &'a self,
        _name: &'a str,
        _input: Value,
        _context: ToolContext<'a>,
    ) -> HostFuture<'a, Result<Value, String>> {
        Box::pin(async { Ok(self.0.clone()) })
    }
}

impl ToolRecovery for Recovery {
    fn recover<'a>(
        &'a self,
        name: &'a str,
        context: ToolContext<'a>,
    ) -> HostFuture<'a, Option<ToolOutput>> {
        Box::pin(async move {
            let (_, content) = self.tools.iter().find(|(candidate, _)| candidate == name)?;
            let reply = match call(json!({"op": "tool_recover", "name": name, "context": context_value(context).await})).await {
                Ok(reply) => reply,
                Err(error) => {
                    let detail = error.get("detail").and_then(Value::as_str).unwrap_or("the host callback failed");
                    return Some(ToolOutput::error(format!("Host tool recovery failed: {detail}. The operation's outcome is unknown; do not automatically repeat it.")));
                }
            };
            if reply.is_null() {
                return None;
            }
            if let Some(error) = reply.get("error").and_then(Value::as_str) {
                return Some(ToolOutput::error(error));
            }
            let Some(result) = reply.get("result") else {
                return Some(ToolOutput::error(
                    "The host returned an invalid tool recovery response. The operation's outcome is unknown; do not automatically repeat it.",
                ));
            };
            let formatter = FunctionTool::new(
                name,
                "",
                Value::Null,
                Arc::new(RecoveredResult(result.clone())),
            )
            .content_results(*content);
            formatter
                .execute(
                    ToolInput::Function(RawValue::from_string("null".into()).ok()?),
                    context,
                )
                .await
                .ok()
        })
    }
}
