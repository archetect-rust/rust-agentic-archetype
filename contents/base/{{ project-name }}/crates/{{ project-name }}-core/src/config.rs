use anyhow::Result;
use figment::{Figment, providers::{Env, Format, Toml}};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    /// Application name
    #[serde(default = "default_name")]
    pub name: String,
{% if has_http then %}
    /// Enable HTTP/SSE MCP transport
    #[serde(default)]
    pub http_enabled: bool,

    /// HTTP listen port
    #[serde(default = "default_http_port")]
    pub http_port: u16,
{% end %}{% if has_sqlite then %}
    /// SQLite database path
    #[serde(default = "default_db_path")]
    pub database_path: String,
{% end %}{% if has_agent then %}
    /// Model to use for inference
    #[serde(default = "default_model")]
    pub model: String,
{% end %}
}

fn default_name() -> String { "{{ project-title }}".into() }
{% if has_http then %}fn default_http_port() -> u16 { 8080 }
{% end %}{% if has_sqlite then %}fn default_db_path() -> String { "{{ project_name }}.db".into() }
{% end %}{% if has_agent then %}fn default_model() -> String { "claude-sonnet-4-6".into() }
{% end %}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: default_name(),
{% if has_http then %}            http_enabled: false,
            http_port: default_http_port(),
{% end %}{% if has_sqlite then %}            database_path: default_db_path(),
{% end %}{% if has_agent then %}            model: default_model(),
{% end %}        }
    }
}

impl AppConfig {
    /// Load configuration from TOML file and environment variables.
    ///
    /// Priority (later overrides earlier):
    /// 1. Compiled defaults
    /// 2. `{{ project-name }}.toml` in the current directory
    /// 3. Environment variables prefixed with `{{ PROJECT_NAME }}_`
    pub fn load() -> Result<Self> {
        Ok(Figment::new()
            .merge(figment::providers::Serialized::defaults(AppConfig::default()))
            .merge(Toml::file("{{ project-name }}.toml"))
            .merge(Env::prefixed("{{ PROJECT_NAME }}_"))
            .extract()?)
    }

    /// Generate a sample configuration file with comments and defaults.
    pub fn sample_toml() -> String {
        let mut out = String::new();
        out.push_str("# {{ project-title }} Configuration\n");
        out.push_str("#\n");
        out.push_str("# Environment variables override these values.\n");
        out.push_str("# Prefix: {{ PROJECT_NAME }}_  (e.g. {{ PROJECT_NAME }}_NAME)\n\n");
        out.push_str("# Application name\n");
        out.push_str(&format!("name = \"{}\"\n", default_name()));
{% if has_http then %}
        out.push_str("\n# HTTP/SSE MCP transport (disabled by default for security)\n");
        out.push_str("http_enabled = false\n");
        out.push_str(&format!("http_port = {}\n", default_http_port()));
{% end %}{% if has_sqlite then %}
        out.push_str("\n# SQLite database path\n");
        out.push_str(&format!("database_path = \"{}\"\n", default_db_path()));
{% end %}{% if has_agent then %}
        out.push_str("\n# Model for LLM inference (requires ANTHROPIC_API_KEY env var)\n");
        out.push_str(&format!("model = \"{}\"\n", default_model()));
{% end %}
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_populated() {
        let config = AppConfig::default();
        assert_eq!(config.name, "{{ project-title }}");
{% if has_http then %}        assert!(!config.http_enabled);
        assert_eq!(config.http_port, 8080);
{% end %}{% if has_sqlite then %}        assert_eq!(config.database_path, "{{ project_name }}.db");
{% end %}{% if has_agent then %}        assert_eq!(config.model, "claude-sonnet-4-6");
{% end %}    }

    #[test]
    fn sample_toml_is_parseable() {
        let toml_str = AppConfig::sample_toml();
        let parsed: AppConfig = toml::from_str(&toml_str).expect("sample TOML should parse");
        assert_eq!(parsed.name, "{{ project-title }}");
    }

    #[test]
    fn load_uses_defaults_when_no_file() {
        let config = AppConfig::load().expect("load should succeed with defaults");
        assert_eq!(config.name, "{{ project-title }}");
    }
}
