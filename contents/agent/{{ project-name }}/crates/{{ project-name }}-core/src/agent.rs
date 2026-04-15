//! Agent module — rig-based LLM inference.

use std::io::{BufRead, Write};

use anyhow::Result;
use rig::{
    client::{CompletionClient, ProviderClient},
    completion::Prompt,
    providers::anthropic,
};

pub async fn run_agent(model: &str, prompt: Option<&str>) -> Result<()> {
    tracing::info!(model, "initializing agent");

    let client = anthropic::Client::from_env();
    let agent = client.agent(model).build();

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
