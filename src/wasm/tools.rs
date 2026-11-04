use std::sync::Arc;

use nanocode::{
    FunctionHandler, FunctionTool, ToolRecovery,
    tool_contract::{Tool, ToolContext, ToolInput, ToolOutput},
};
use nanocode_code_mode::embedded::HostFuture;
use serde::Deserialize;
use serde_json::{Value, json, value::to_raw_value};

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

struct Recovered;

struct Recovery {
    content: Box<[String]>,
}

async fn described(context: &ToolContext<'_>) -> Value {
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

impl FunctionHandler for Bridge {
    fn call<'a>(
        &'a self,
        name: &'a str,
        input: Value,
        context: ToolContext<'a>,
    ) -> HostFuture<'a, Result<Value, String>> {
        Box::pin(async move {
            let context = described(&context).await;
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

impl FunctionHandler for Recovered {
    fn call<'a>(
        &'a self,
        _name: &'a str,
        input: Value,
        _context: ToolContext<'a>,
    ) -> HostFuture<'a, Result<Value, String>> {
        Box::pin(async move { Ok(input) })
    }
}

impl ToolRecovery for Recovery {
    fn recover<'a>(
        &'a self,
        name: &'a str,
        context: ToolContext<'a>,
    ) -> HostFuture<'a, Option<ToolOutput>> {
        Box::pin(async move {
            let described = described(&context).await;
            let result =
                match call(json!({"op": "tool_recover", "name": name, "context": described})).await
                {
                    Ok(Value::Null) | Err(_) => return None,
                    Ok(result) => to_raw_value(&result).ok()?,
                };
            let content = self.content.iter().any(|tool| tool == name);
            FunctionTool::new(name, "", json!({}), Arc::new(Recovered))
                .content_results(content)
                .execute(ToolInput::Function(result), context)
                .await
                .ok()
        })
    }
}

impl HostTool {
    pub fn name(&self) -> &str {
        &self.name
    }
}

pub fn recovery(definitions: &[HostTool]) -> Arc<dyn ToolRecovery> {
    Arc::new(Recovery {
        content: definitions
            .iter()
            .filter(|tool| tool.content)
            .map(|tool| tool.name.clone())
            .collect(),
    })
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
