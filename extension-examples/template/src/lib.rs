use omni_sdk::prelude::*;

#[derive(Default)]
struct MyExtension;

impl Extension for MyExtension {
    fn handle_tool(
        &mut self,
        ctx: &Context,
        tool_name: &str,
        params: serde_json::Value,
    ) -> ToolResult {
        match tool_name {
            "hello" => {
                let name = params["name"]
                    .as_str()
                    .ok_or_else(|| SdkError::Other("Missing 'name' parameter".into()))?;

                ctx.info(&format!("Greeting {name}"));

                Ok(serde_json::json!({
                    "greeting": format!("Hello, {name}!"),
                }))
            }
            _ => Err(SdkError::UnknownTool(tool_name.to_string())),
        }
    }
}

omni_sdk::omni_main!(MyExtension);
