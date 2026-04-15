//! Agent module — rig-based LLM inference with tool support.
//!
//! Tools are registered as native rig `Tool` implementations, sharing business
//! logic with the MCP server tools. This avoids coupling the agent to a specific
//! rmcp version while keeping tool behavior consistent.

use std::io::{BufRead, Write};

use anyhow::Result;
use rig::{
    client::{CompletionClient, ProviderClient},
    completion::{Prompt, ToolDefinition},
    providers::anthropic,
    tool::Tool,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

// ── Agent Tools ──────────────────────────────────────────────────────────
//
// Each tool is a struct implementing rig's `Tool` trait. For every MCP tool
// defined in `server.rs`, create a matching rig tool here so the agent can
// use the same capabilities.

/// Echo tool — mirrors the MCP server's echo tool.
pub struct EchoTool;

#[derive(Deserialize)]
pub struct EchoArgs {
    /// The message to echo back.
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct EchoError(String);

#[derive(Serialize)]
pub struct EchoOutput {
    pub message: String,
}

impl Tool for EchoTool {
    const NAME: &'static str = "echo";

    type Error = EchoError;
    type Args = EchoArgs;
    type Output = EchoOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "echo".to_string(),
            description: "Echoes the input message back to the caller".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "The message to echo back"
                    }
                },
                "required": ["message"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(EchoOutput {
            message: args.message,
        })
    }
}

// ── Agent Runner ─────────────────────────────────────────────────────────

/// Run the agent with registered tools.
pub async fn run_agent(model: &str, prompt: Option<&str>) -> Result<()> {
    tracing::info!(model, "initializing agent with tools");

    let client = anthropic::Client::from_env();
    let agent = client
        .agent(model)
        .preamble("You are a helpful assistant. You have access to tools — use them when appropriate.")
        .tool(EchoTool)
        // Register additional tools here as they are added to the MCP server.
        .build();

    match prompt {
        Some(prompt) => {
            let response = agent.prompt(prompt).await?;
            println!("{response}");
        }
        None => {
            println!("Interactive mode (type 'exit' to quit)");
            let stdin = std::io::stdin();
            let mut stdout = std::io::stdout();

            loop {
                print!("> ");
                stdout.flush()?;

                let mut input = String::new();
                if stdin.lock().read_line(&mut input)? == 0 {
                    break;
                }

                let input = input.trim();
                if input.is_empty() {
                    continue;
                }
                if input == "exit" || input == "quit" {
                    break;
                }

                match agent.prompt(input).await {
                    Ok(response) => println!("{response}"),
                    Err(e) => eprintln!("Error: {e}"),
                }
            }
        }
    }

    Ok(())
}
