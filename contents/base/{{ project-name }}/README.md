# {{ project-title }}

> Generated with [archetect](https://archetect.github.io) — Agentic Rust Archetype

## Features

- **MCP Server** (STDIO{% if has_http then %}, HTTP/SSE{% end %})
{% if has_agent then %}- **Agent** — rig-based LLM inference
{% end %}{% if has_sqlite then %}- **SQLite** — Persistent storage
{% end %}

## Getting Started

```bash
cargo run -- mcp
```
{% if has_http then %}
To also expose HTTP/SSE (disabled by default):

```bash
cargo run -- mcp --http
```
{% end %}

### Configuration

Generate a sample config file with defaults:

```bash
cargo run -- config generate > {{ project-name }}.toml
```

View the effective (merged) configuration:

```bash
cargo run -- config show
```
