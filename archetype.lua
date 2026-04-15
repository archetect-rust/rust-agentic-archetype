local context = Context.new()

-- ── License (pre-populated so author-prompts skips its own prompt) ───
-- Override the default "None at the bottom" ordering from author-prompts
-- so -D / --use-defaults-all selects "None".
context:prompt_select("License:", "license",
    { "None", "Apache-2.0", "MIT", "GPL-3.0", "BSD-3-Clause" },
    { default = "None" })

-- ── Author + license (composed from author-prompts) ──────────────────
-- license is already set above; author-prompts will skip it.
context:merge(catalog.render("author-prompts", context))

-- ── Features ─────────────────────────────────────────────────────────
context:prompt_multiselect("Features:", "features", { "Agent", "HTTP MCP", "SQLite", "xtask" })

context:set("has_mcp", true)
context:set("has_stdio", true)
context:set("has_agent", context:contains("features", "Agent"))
context:set("has_http", context:contains("features", "HTTP MCP"))
context:set("has_sqlite", context:contains("features", "SQLite"))
context:set("has_xtask", context:contains("features", "xtask"))

-- ── Project Suffix (auto-determined from features) ───────────────────
-- Pre-populate suffix_name before calling project-prompts so its suffix
-- prompt is skipped. We keep a hardcoded suffix_title so "MCP" stays in
-- the correct acronym casing (Title-case would produce "Mcp").
local suffix_title
if context:get("has_agent") then
    context:set("suffix_name", "agent", { cases = Cases.programming() })
    suffix_title = "Agent"
else
    context:set("suffix_name", "mcp", { cases = Cases.programming() })
    suffix_title = "MCP"
end

-- ── Project Prefix (composed from project-prompts) ───────────────────
-- project-prompts prompts for prefix_name, skips suffix (already set),
-- and composes project_name with Cases.programming() + project_title.
context:merge(catalog.render("project-prompts", context))

-- Templates reference `{{ project-title }}` (kebab); compose it here
-- using the fixed-cased prefix_title and our acronym-preserving suffix.
context:set("project-title", context:get("prefix_title") .. " " .. suffix_title)

-- ── Render ───────────────────────────────────────────────────────────

-- Base workspace (always rendered)
directory.render("contents/base", context)

-- MCP server + stdio transport (always rendered)
directory.render("contents/mcp", context)
directory.render("contents/mcp-stdio", context)

-- HTTP/SSE transport (optional)
if context:get("has_http") then
	directory.render("contents/mcp-http", context)
end

-- Agent support
if context:get("has_agent") then
	directory.render("contents/agent", context)
	directory.render("contents/agent-mcp", context)
end

-- SQLite persistence
if context:get("has_sqlite") then
	directory.render("contents/sqlite", context)
end

-- xtask
if context:get("has_xtask") then
	directory.render("contents/crate-xtask", context)
end

-- Gitignore (composed component — rendered into project subdirectory).
-- The dot-gitignore archetype prompts for the key `ignores` (plural);
-- pre-populate it so it renders non-interactively under our selection.
context:set("ignores", { "IDEA", "VSCode", "Eclipse", "Claude", "Rust" })
catalog.render("gitignore", context, {
    destination = context:get("project-name"),
})

-- ── Post-generation guidance ─────────────────────────────────────────

output.print("")
log.info("Your project has been generated!")
output.print("")
log.info("Next steps:")
output.print(template.render("  cd {{ project-name }}", context))
output.print("  cargo build")
output.print("  cargo run -- mcp             # MCP over stdio")
if context:get("has_agent") then
	output.print("  cargo run -- agent            # Interactive agent")
	output.print("  cargo run -- agent -p '...'   # One-shot prompt")
end
output.print("")
