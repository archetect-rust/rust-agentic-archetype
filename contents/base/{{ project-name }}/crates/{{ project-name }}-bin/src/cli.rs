use clap::{Parser, Subcommand, ValueEnum};

/// {{ project-title }}
#[derive(Parser)]
#[command(name = "{{ project-name }}", version)]
pub struct Cli {
    /// Increase logging verbosity (-v, -vv, -vvv)
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress all output except errors
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Log format
    #[arg(long, global = true, default_value = "auto", env = "{{ PROJECT_NAME }}_LOG_FORMAT")]
    pub log_format: LogFormat,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run as MCP server (stdio transport{% if has_http then %}, with optional HTTP{% end %})
    Mcp {
{% if has_http then %}        /// Enable HTTP/SSE transport (overrides config)
        #[arg(long, env = "{{ PROJECT_NAME }}_HTTP_ENABLED")]
        http: bool,

        /// HTTP listen port (implies --http)
        #[arg(long, env = "{{ PROJECT_NAME }}_HTTP_PORT")]
        http_port: Option<u16>,
{% end %}    },
{% if has_agent then %}
    /// Run the agent
    Agent {
        /// Initial prompt (interactive if omitted)
        #[arg(short, long)]
        prompt: Option<String>,
    },
{% end %}
    /// Configuration utilities
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Print a sample config file with defaults and comments
    Generate,
    /// Print the effective (merged) configuration
    Show,
}

#[derive(Clone, ValueEnum)]
pub enum LogFormat {
    /// Pretty if TTY, compact otherwise
    Auto,
    /// Human-readable with colors
    Pretty,
    /// Single-line, minimal formatting
    Compact,
    /// Structured JSON for log aggregation
    Json,
}
