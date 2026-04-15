# {{ project-title }}

## Build & Run

```bash
cargo build                     # Build workspace
cargo run -- --help             # Show CLI help
cargo run -- mcp                # MCP over stdio
{% if has_http then %}cargo run -- mcp --http         # MCP over stdio + HTTP (port 8080)
{% end %}{% if has_agent then %}cargo run -- agent              # Interactive agent
cargo run -- agent -p '...'    # One-shot prompt
{% end %}cargo run -- config generate    # Print sample config TOML
cargo run -- config show        # Print effective merged config
cargo test --workspace          # Run all tests
cargo clippy --all-targets     # Run lints
cargo fmt --all                # Format code
{% if has_xtask then %}cargo xtask install            # Install binary to ~/.cargo/bin
cargo xtask run -- mcp         # Run via xtask (pass any args after --)
cargo xtask build              # Build release binary
cargo xtask check              # Check all targets
cargo xtask clippy             # Clippy with -D warnings
cargo xtask fmt                # Format code
cargo xtask fmt --check        # Check formatting without modifying
cargo xtask test-ut            # Run unit tests only
cargo xtask test-it            # Run integration tests only
cargo xtask test-all           # Run all tests
cargo xtask sweep              # Clean stale build artifacts (>7 days)
{% end %}```

## Project Structure

```
crates/
  {{ project-name }}-bin/              CLI binary
    src/
      main.rs                          Entry point, subcommand dispatch
      cli.rs                           Clap CLI definition
      logging.rs                       Tracing/logging setup
  {{ project-name }}-core/             Core library
    src/
      lib.rs                           Module exports
      config.rs                        AppConfig (figment TOML + env)
      error.rs                         AppError (thiserror)
      server.rs                        MCP server + tool definitions
      transport_stdio.rs               Stdio MCP transport
{% if has_http then %}      transport_http.rs                HTTP MCP transport + /health
{% end %}{% if has_agent then %}      agent.rs                         Rig agent + tool impls
{% end %}{% if has_sqlite then %}      db.rs                            SQLite with versioned migrations
{% end %}{% if has_xtask then %}xtask/                               Build automation (cargo xtask <cmd>)
{% end %}```

## Key Patterns

### Logging
All output goes to stderr (stdout is reserved for MCP protocol on stdio). Control via `RUST_LOG` env or `-v`/`-q` flags. `--log-format` supports auto/pretty/compact/json.

### Error Handling
`anyhow::Result` for application errors, `thiserror` for typed domain errors in `error.rs`.

### Configuration
figment loads config with this priority (later overrides earlier):
1. Compiled defaults in `AppConfig::default()`
2. `{{ project-name }}.toml` in the working directory
3. Environment variables prefixed with `{{ PROJECT_NAME }}_`

Config is wrapped in `Arc<AppConfig>` and passed to the server constructor. Run `cargo run -- config generate > {{ project-name }}.toml` to create a starter config file.

**To add a config field:**
1. Add the field to `AppConfig` in `config.rs` with `#[serde(default = "default_fn")]`
2. Add the default function and update `Default` impl
3. Add a line to `sample_toml()`
4. Access via `self.config.field_name` in server tools or agent

### MCP Tools
Tools are defined as methods on `{{ ProjectName }}Server` in `core/src/server.rs`.

**To add a new tool:**
1. Define an input struct deriving `Deserialize` + `schemars::JsonSchema`. Document fields with `///` doc comments (these become the parameter descriptions in the tool schema).
2. Add an async method in the `#[tool_router] impl` block with `#[tool(description = "...")]`
3. Use `Parameters<YourInput>` as the parameter type
4. Return `Result<CallToolResult, ErrorData>`
{% if has_agent then %}5. Create a matching rig `Tool` impl in `agent.rs` (see below){% end %}

```rust
#[derive(Deserialize, schemars::JsonSchema)]
struct MyInput {
    /// Description for the LLM.
    field: String,
}

#[tool(description = "What this tool does")]
async fn my_tool(
    &self,
    Parameters(MyInput { field }): Parameters<MyInput>,
) -> Result<CallToolResult, ErrorData> {
    // Access config: self.config.some_field
    Ok(CallToolResult::success(vec![Content::text("result")]))
}
```
{% if has_agent then %}
### Agent & Tool Registration

The agent uses rig-core with the Anthropic provider. Set `ANTHROPIC_API_KEY` env var.

Agent tools are native rig `Tool` trait impls in `core/src/agent.rs`. For each MCP tool in `server.rs`, create a matching rig tool so the agent has the same capabilities.

**To add an agent tool:**
1. Define `Args` (Deserialize), `Output` (Serialize), and `Error` types
2. Implement `rig::tool::Tool` with `NAME`, `definition()`, and `call()`
3. Register with `.tool(MyTool)` on the agent builder in `run_agent()`
{% end %}{% if has_sqlite then %}
### SQLite & Migrations
Database uses rusqlite with WAL mode. Schema is versioned with `PRAGMA user_version`.

**To add a migration:**
1. Increment `CURRENT_VERSION` in `db.rs`
2. Add a `SCHEMA_V{N}` constant with the migration SQL
3. Add `if from_version < N { tx.execute_batch(SCHEMA_V{N})?; }` in `apply_migrations()`

Migrations run automatically on `Database::open()`. Use `:memory:` for tests.
{% end %}{% if has_http then %}
### HTTP Transport
axum server with graceful shutdown (Ctrl+C + SIGTERM). **Disabled by default** for security.

Enable via config (`http_enabled = true`), env var (`{{ PROJECT_NAME }}_HTTP_ENABLED=true`), or CLI (`--http` / `--http-port`). Endpoints:
- `GET /health` — JSON health status (name from config, version from Cargo.toml)
- `/mcp` — MCP streamable HTTP transport

CORS is permissive by default. Tighten `CorsLayer` in `transport_http.rs` for production.

The HTTP task is spawned alongside stdio and shares its lifetime. When the MCP client disconnects and stdio exits, the process terminates and the HTTP task is dropped. If you need the HTTP transport to outlive stdio, add a `CancellationToken` and coordinate shutdown explicitly in `main`.
{% end %}

## Specifications

Feature and design specs live in `docs/specs/`. Each spec is a Markdown file covering motivation, design decisions, data shapes, and open questions. Before implementing a non-trivial feature, write or reference the spec.

See [docs/specs/INDEX.md](docs/specs/INDEX.md) for the current spec registry.

## Cargo Rules

- **Never run concurrent cargo commands.** Cargo subcommands share the `target/` lock — running two at once causes deadlocks and multi-minute hangs.
- **Lints**: `unsafe_code` is denied. Clippy warnings are enabled workspace-wide.
- **Edition**: Rust 2024.
- **After any code change**, run `cargo test --workspace` to verify.

## Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| rmcp | 0.17 | MCP server, tool macros, transports |
| reqwest | 0.12 | HTTP client (rustls-tls, json, stream) |
{% if has_agent then %}| rig-core | 0.31 | LLM agent framework (Anthropic provider) |
{% end %}| tokio | 1 | Async runtime |
| clap | 4 | CLI parsing |
| figment | 0.10 | Configuration (TOML + env) |
{% if has_http then %}| axum | 0.8 | HTTP server |
{% end %}{% if has_sqlite then %}| rusqlite | 0.38 | SQLite (bundled, WAL mode) |
{% end %}| serde | 1.0 | Serialization |
| anyhow | 1 | Error handling |
| thiserror | 2 | Typed errors |
| tracing | 0.1 | Structured logging |
