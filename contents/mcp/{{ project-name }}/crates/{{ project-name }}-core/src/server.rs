//! MCP server handler.
//!
//! Implements the rmcp `ServerHandler` trait with tool routing.
//! Add new tools as async methods with `#[tool(description = "...")]`.

use std::sync::Arc;

use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars,
    tool, tool_handler, tool_router,
};
use serde::Deserialize;

use crate::config::AppConfig;

#[derive(Clone)]
pub struct {{ ProjectName }}Server {
    tool_router: ToolRouter<Self>,
    pub config: Arc<AppConfig>,
}

impl {{ ProjectName }}Server {
    pub fn new(config: Arc<AppConfig>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            config,
        }
    }
}

// ── Tools ────────────────────────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema)]
struct EchoInput {
    /// The message to echo back.
    message: String,
}

#[tool_router]
impl {{ ProjectName }}Server {
    #[tool(description = "Echoes the input message back to the caller")]
    async fn echo(
        &self,
        Parameters(EchoInput { message }): Parameters<EchoInput>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(CallToolResult::success(vec![Content::text(message)]))
    }
}

// ── Handler ──────────────────────────────────────────────────────────────

#[tool_handler]
impl ServerHandler for {{ ProjectName }}Server {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(self.config.name.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_server() -> {{ ProjectName }}Server {
        {{ ProjectName }}Server::new(Arc::new(AppConfig::default()))
    }

    #[tokio::test]
    async fn echo_returns_input() {
        let server = test_server();
        let input = Parameters(EchoInput {
            message: "hello world".into(),
        });
        let result = server.echo(input).await.expect("echo should succeed");
        assert!(!result.is_error.unwrap_or(false));
        let json = serde_json::to_string(&result.content[0]).unwrap();
        assert!(json.contains("hello world"), "expected 'hello world' in {json}");
    }

    #[test]
    fn server_info_has_tools_capability() {
        let server = test_server();
        let info = server.get_info();
        assert!(info.capabilities.tools.is_some());
    }
}
