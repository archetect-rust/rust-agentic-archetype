use std::sync::Arc;

use anyhow::Result;
use clap::Parser;

mod cli;
mod logging;

use cli::{Cli, Commands, ConfigAction};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    logging::init(cli.verbose, cli.quiet, cli.log_format.clone());

    match cli.command {
        Commands::Mcp { {% if has_http then %}http, http_port{% end %} } => {
            let config = Arc::new({{ project_name }}_core::config::AppConfig::load()?);
            let server = {{ project_name }}_core::server::{{ ProjectName }}Server::new(Arc::clone(&config));
{% if has_http then %}
            // HTTP is enabled by CLI flag, explicit port, or config
            let http_enabled = http || http_port.is_some() || config.http_enabled;
            if http_enabled {
                let port = http_port.unwrap_or(config.http_port);
                let http_server = server.clone();
                tokio::spawn(async move {
                    if let Err(e) = {{ project_name }}_core::transport_http::serve_http(http_server, port).await {
                        tracing::error!("HTTP transport error: {e}");
                    }
                });
            }
{% end %}
            {{ project_name }}_core::transport_stdio::serve_stdio(server).await
        }
{% if has_agent then %}
        Commands::Agent { prompt } => {
            let config = {{ project_name }}_core::config::AppConfig::load()?;
            {{ project_name }}_core::agent::run_agent(&config.model, prompt.as_deref()).await
        }
{% end %}
        Commands::Config { action } => {
            match action {
                ConfigAction::Generate => {
                    print!("{}", {{ project_name }}_core::config::AppConfig::sample_toml());
                    Ok(())
                }
                ConfigAction::Show => {
                    let config = {{ project_name }}_core::config::AppConfig::load()?;
                    println!("{}", toml::to_string_pretty(&config)?);
                    Ok(())
                }
            }
        }
    }
}
