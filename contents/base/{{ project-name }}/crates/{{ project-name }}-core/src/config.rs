use anyhow::Result;
use figment::{Figment, providers::{Env, Format, Toml, Yaml}};
use serde::{Deserialize, Serialize};
{% if has_http then %}
// ── HTTP config ───────────────────────────────────────────────────────────────

/// OAuth 2.0 / OIDC configuration for the external HTTP endpoint.
///
/// When present on [`HttpConfig::oauth`], the `/mcp` route requires a valid
/// Bearer JWT. `/health` and `/.well-known/oauth-authorization-server` remain
/// public.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OAuthConfig {
    /// Authorization server issuer URL (e.g. `"https://auth.example.com"`).
    /// Used to validate the `iss` claim and to discover the JWKS URI.
    pub issuer: String,

    /// Expected `aud` claim in the JWT. If omitted, audience validation is skipped.
    #[serde(default)]
    pub audience: Option<String>,

    /// Override the JWKS URI. Defaults to `{issuer}/.well-known/jwks.json`.
    #[serde(default)]
    pub jwks_uri: Option<String>,
}

impl OAuthConfig {
    pub fn effective_jwks_uri(&self) -> String {
        self.jwks_uri.clone().unwrap_or_else(|| {
            format!("{}/.well-known/jwks.json", self.issuer.trim_end_matches('/'))
        })
    }

    pub fn oidc_discovery_uri(&self) -> String {
        format!("{}/.well-known/openid-configuration", self.issuer.trim_end_matches('/'))
    }
}

/// Internal (unauthenticated) HTTP endpoint config.
///
/// Bound to loopback. Intended for same-host callers that cannot present an
/// OAuth token. Do not expose publicly.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InternalHttpConfig {
    /// Enable the internal HTTP endpoint.
    #[serde(default)]
    pub enabled: bool,

    /// Port to listen on. Defaults to 8081.
    #[serde(default = "default_internal_port")]
    pub port: u16,
}

fn default_internal_port() -> u16 { 8081 }

impl Default for InternalHttpConfig {
    fn default() -> Self {
        Self { enabled: false, port: default_internal_port() }
    }
}

/// HTTP transport configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HttpConfig {
    /// Enable the external HTTP endpoint.
    #[serde(default)]
    pub enabled: bool,

    /// External HTTP listen port. Defaults to 8080.
    #[serde(default = "default_http_port")]
    pub port: u16,

    /// If present, the external endpoint requires a valid Bearer JWT.
    /// The `/.well-known/oauth-authorization-server` and `/health` endpoints
    /// are always public regardless of this setting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth: Option<OAuthConfig>,

    /// Internal (unauthenticated) HTTP endpoint.
    #[serde(default)]
    pub internal: InternalHttpConfig,
}

fn default_http_port() -> u16 { 8080 }

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: default_http_port(),
            oauth: None,
            internal: InternalHttpConfig::default(),
        }
    }
}
{% end %}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    /// Application name
    #[serde(default = "default_name")]
    pub name: String,
{% if has_http then %}
    /// HTTP transport configuration.
    #[serde(default)]
    pub http: HttpConfig,
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
{% if has_sqlite then %}fn default_db_path() -> String { "{{ project_name }}.db".into() }
{% end %}{% if has_agent then %}fn default_model() -> String { "claude-sonnet-4-6".into() }
{% end %}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: default_name(),
{% if has_http then %}            http: HttpConfig::default(),
{% end %}{% if has_sqlite then %}            database_path: default_db_path(),
{% end %}{% if has_agent then %}            model: default_model(),
{% end %}        }
    }
}

impl AppConfig {
    /// Load configuration from TOML/YAML files and environment variables.
    ///
    /// Both formats are supported; YAML takes precedence over TOML when both
    /// define the same key. Priority (later overrides earlier):
    /// 1. Compiled defaults
    /// 2. `{{ project-name }}.toml` next to the binary
    /// 3. `{{ project-name }}.toml` in the current directory
    /// 4. `{{ project-name }}.yaml` / `.yml` next to the binary
    /// 5. `{{ project-name }}.yaml` / `.yml` in the current directory
    /// 6. Environment variables prefixed with `{{ PROJECT_NAME }}_`
    pub fn load() -> Result<Self> {
        let bin_dir = std::env::current_exe().ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()));

        let mut figment = Figment::new()
            .merge(figment::providers::Serialized::defaults(AppConfig::default()));

        if let Some(dir) = &bin_dir {
            figment = figment.merge(Toml::file(dir.join("{{ project-name }}.toml")));
        }
        figment = figment.merge(Toml::file("{{ project-name }}.toml"));

        if let Some(dir) = &bin_dir {
            figment = figment
                .merge(Yaml::file(dir.join("{{ project-name }}.yaml")))
                .merge(Yaml::file(dir.join("{{ project-name }}.yml")));
        }
        figment = figment
            .merge(Yaml::file("{{ project-name }}.yaml"))
            .merge(Yaml::file("{{ project-name }}.yml"));

        Ok(figment
            .merge(Env::prefixed("{{ PROJECT_NAME }}_"))
            .extract()?)
    }

    /// Generate a sample configuration file in YAML format.
    pub fn sample_yaml() -> String {
        let default = AppConfig::default();
        let mut out = String::new();
        out.push_str("# {{ project-title }} Configuration\n");
        out.push_str("#\n");
        out.push_str("# Environment variables override these values.\n");
        out.push_str("# Prefix: {{ PROJECT_NAME }}_  (e.g. {{ PROJECT_NAME }}_NAME)\n\n");
        out.push_str(&serde_yml::to_string(&default)
            .unwrap_or_else(|e| format!("# failed to serialize defaults: {e}\n")));
{% if has_http then %}        out.push_str("\n# OAuth 2.0 protection for the external endpoint.\n");
        out.push_str("# When present, /mcp requires a valid Bearer JWT.\n");
        out.push_str("# /health and /.well-known/oauth-authorization-server remain public.\n");
        out.push_str("#\n");
        out.push_str("# http:\n");
        out.push_str("#   oauth:\n");
        out.push_str("#     issuer: \"https://auth.example.com\"\n");
        out.push_str("#     audience: \"{{ project-name }}\"\n");
{% end %}        out
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
        out.push_str("\n# HTTP transport\n");
        out.push_str("[http]\n");
        out.push_str("# External endpoint — disabled by default for security\n");
        out.push_str("enabled = false\n");
        out.push_str(&format!("port = {}\n", default_http_port()));
        out.push_str("\n# OAuth 2.0 protection for the external endpoint.\n");
        out.push_str("# When present, /mcp requires a valid Bearer JWT.\n");
        out.push_str("# /health and /.well-known/oauth-authorization-server are always public.\n");
        out.push_str("# [http.oauth]\n");
        out.push_str("# issuer = \"https://auth.example.com\"  # Authorization server issuer URL\n");
        out.push_str("# audience = \"{{ project-name }}\"       # Expected 'aud' claim (optional)\n");
        out.push_str("\n# Internal endpoint — no auth, loopback only\n");
        out.push_str("[http.internal]\n");
        out.push_str("enabled = false\n");
        out.push_str(&format!("port = {}\n", default_internal_port()));
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
{% if has_http then %}        assert!(!config.http.enabled);
        assert_eq!(config.http.port, 8080);
        assert!(!config.http.internal.enabled);
        assert_eq!(config.http.internal.port, 8081);
        assert!(config.http.oauth.is_none());
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
    fn sample_yaml_is_parseable() {
        let yaml_str = AppConfig::sample_yaml();
        let parsed: AppConfig = serde_yml::from_str(&yaml_str).expect("sample YAML should parse");
        assert_eq!(parsed.name, "{{ project-title }}");
    }

    #[test]
    fn load_uses_defaults_when_no_file() {
        let config = AppConfig::load().expect("load should succeed with defaults");
        assert_eq!(config.name, "{{ project-title }}");
    }
{% if has_http then %}
    #[test]
    fn oauth_config_round_trips_yaml() {
        let mut config = AppConfig::default();
        config.http.enabled = true;
        config.http.oauth = Some(OAuthConfig {
            issuer: "https://auth.example.com".into(),
            audience: Some("svc".into()),
            jwks_uri: None,
        });
        let yaml = serde_yml::to_string(&config).expect("serialize");
        let parsed: AppConfig = serde_yml::from_str(&yaml).expect("parse");
        let oauth = parsed.http.oauth.expect("oauth should round-trip");
        assert_eq!(oauth.issuer, "https://auth.example.com");
        assert_eq!(oauth.audience.as_deref(), Some("svc"));
    }

    #[test]
    fn oauth_config_parses_from_toml() {
        let toml_str = r#"
            name = "Test"

            [http]
            enabled = true
            port = 8080

            [http.oauth]
            issuer = "https://auth.example.com"
            audience = "my-service"

            [http.internal]
            enabled = true
            port = 8081
        "#;
        let config: AppConfig = toml::from_str(toml_str).expect("should parse");
        assert!(config.http.enabled);
        let oauth = config.http.oauth.expect("oauth should be present");
        assert_eq!(oauth.issuer, "https://auth.example.com");
        assert_eq!(oauth.audience.as_deref(), Some("my-service"));
        assert_eq!(oauth.effective_jwks_uri(), "https://auth.example.com/.well-known/jwks.json");
        assert!(config.http.internal.enabled);
        assert_eq!(config.http.internal.port, 8081);
    }
{% end %}}
