# {{ project-title }}

> Generated with [archetect](https://archetect.github.io) — Agentic Rust Archetype

## Features

- **MCP Server** (STDIO{% if has_http then %}, HTTP with optional OAuth, loopback HTTP{% end %})
{% if has_agent then %}- **Agent** — rig-based LLM inference
{% end %}{% if has_sqlite then %}- **SQLite** — Persistent storage
{% end %}

## Getting Started

```bash
cargo run -- mcp
```
{% if has_http then %}
To also expose the external HTTP endpoint (disabled by default):

```bash
cargo run -- mcp --http                # port from config, default 8080
cargo run -- mcp --internal-http       # loopback endpoint, no auth
```

When `[http.oauth]` is set in the config, `/mcp` requires a valid Bearer JWT; `/health` and `/.well-known/oauth-authorization-server` stay public.
{% end %}

### Configuration

Both YAML and TOML are supported; YAML takes precedence on conflicting keys.

```bash
cargo run -- config generate > {{ project-name }}.yaml          # YAML (default)
cargo run -- config generate --format toml > {{ project-name }}.toml
cargo run -- config show                                         # print merged config
```

Environment variables prefixed with `{{ PROJECT_NAME }}_` override file values. A `.env` file is loaded from the working directory and the binary's directory at startup — add `.env` to `.gitignore` before checking in secrets.
