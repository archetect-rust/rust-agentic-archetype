use std::sync::Arc;

use anyhow::Result;
use clap::Parser;

mod cli;
mod logging;

use cli::{Cli, Commands, ConfigAction, ConfigFormat};

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env — try CWD first (dev), then the binary's own directory (when
    // launched by Claude Desktop or another host that sets a different CWD).
    dotenvy::dotenv().ok();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            dotenvy::from_path(dir.join(".env")).ok();
        }
    }

    let cli = Cli::parse();
    logging::init(cli.verbose, cli.quiet, cli.log_format.clone());

    match cli.command {
        Commands::Mcp { {% if has_http then %}http, http_port, internal_http, internal_http_port{% end %} } => {
            let config = Arc::new({{ project_name }}_core::config::AppConfig::load()?);
            let server = {{ project_name }}_core::server::{{ ProjectName }}Server::new(Arc::clone(&config));
{% if has_http then %}
            // External HTTP — enabled by CLI flag, explicit port, or config
            let ext_enabled = http || http_port.is_some() || config.http.enabled;
            if ext_enabled {
                let port = http_port.unwrap_or(config.http.port);
                let oauth = config.http.oauth.clone();
                let http_server = server.clone();
                tokio::spawn(async move {
                    if let Err(e) = {{ project_name }}_core::transport_http::serve_http(http_server, port, oauth).await {
                        tracing::error!("external HTTP transport error: {e}");
                    }
                });
            }

            // Internal HTTP — enabled by CLI flag, explicit port, or config
            let int_enabled = internal_http || internal_http_port.is_some() || config.http.internal.enabled;
            if int_enabled {
                let port = internal_http_port.unwrap_or(config.http.internal.port);
                let internal_server = server.clone();
                tokio::spawn(async move {
                    if let Err(e) = {{ project_name }}_core::transport_http::serve_internal_http(internal_server, port).await {
                        tracing::error!("internal HTTP transport error: {e}");
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
                ConfigAction::Generate { format } => {
                    let sample = match format {
                        ConfigFormat::Yaml => {{ project_name }}_core::config::AppConfig::sample_yaml(),
                        ConfigFormat::Toml => {{ project_name }}_core::config::AppConfig::sample_toml(),
                    };
                    print!("{sample}");
                    Ok(())
                }
                ConfigAction::Show { format } => {
                    let config = {{ project_name }}_core::config::AppConfig::load()?;
                    let out = match format {
                        ConfigFormat::Yaml => serde_yml::to_string(&config)?,
                        ConfigFormat::Toml => toml::to_string_pretty(&config)?,
                    };
                    print!("{out}");
                    Ok(())
                }
            }
        }
    }
}
