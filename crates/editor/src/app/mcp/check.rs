//! `axcenity-editor --mcp --check <project>`: says whether the server would
//! work for this project, without serving it, and prints the lines to add
//! it to the common clients, with this binary's and the project's full
//! paths filled in (`docs/mcp/README.md`).
//!
//! Each finding is a line starting `ok`, `warn` or `FAIL`; any `FAIL`
//! makes the command exit with 1. It writes nothing to the project but a
//! probe file it deletes again (to know the call log can be written).

use std::path::{Path, PathBuf};

use axcenity_mcp::{Args, Profile, text::estimate_tokens, tools_list_json};

use super::{Headless, INSTRUCTIONS, LOG_FILE, Options, TOOLKIT_VERSION, project_info, tools_for};
use crate::app::EditorApp;

/// The report for `options`, and whether nothing failed.
pub(super) fn report(options: &Options, exe: &Path) -> (String, bool) {
    let mut lines = vec!["Axcenity MCP check".to_owned()];
    let mut failed = false;
    let mut note = |level: &str, text: String| {
        failed |= level == "FAIL";
        lines.push(format!("{level:<4} {text}"));
    };
    let folder = std::fs::canonicalize(&options.folder).unwrap_or_else(|_| options.folder.clone());
    let app = EditorApp::new(Some(&folder));
    let opened = app.project.is_some();
    if opened {
        let mut headless = Headless::new(app, options.mode);
        match project_info::call(&mut headless.host(), &Args::default()) {
            Ok(info) => {
                for line in info.lines() {
                    note("ok", line.to_owned());
                }
            }
            Err(error) => note("FAIL", error.to_string()),
        }
        match log_problem(&folder) {
            Some(problem) => note("FAIL", problem),
            None => note(
                "ok",
                format!("calls are logged to {}", folder.join(LOG_FILE).display()),
            ),
        }
    } else {
        let why = app
            .output
            .lines()
            .last()
            .map_or_else(String::new, |line| format!(": {}", line.text));
        note(
            "ok",
            format!(
                "engine {} · toolkit {TOOLKIT_VERSION}",
                env!("CARGO_PKG_VERSION")
            ),
        );
        note(
            "FAIL",
            format!("could not open a project in {}{why}", folder.display()),
        );
    }
    let tools = tools_for(options.profile);
    let tokens = estimate_tokens(&tools_list_json(&tools)) + estimate_tokens(INSTRUCTIONS);
    note(
        "ok",
        format!(
            "profile {}: {} tools, about {tokens} tokens of definitions and instructions",
            options.profile.name(),
            tools.len()
        ),
    );
    for import in &options.import_folders {
        if import.is_dir() {
            note(
                "ok",
                format!("asset_import may copy from {}", import.display()),
            );
        } else {
            note(
                "FAIL",
                format!("--allow-import {}: not a folder", import.display()),
            );
        }
    }
    lines.push(String::new());
    lines.push(client_lines(exe, &folder, options));
    (lines.join("\n"), !failed)
}

/// Why the call log couldn't be written, if it can't: a probe file is made
/// next to it and removed.
fn log_problem(folder: &Path) -> Option<String> {
    let log = folder.join(LOG_FILE);
    let dir = log.parent()?;
    let probe = dir.join(".mcp-check");
    let result = std::fs::create_dir_all(dir).and_then(|()| std::fs::write(&probe, b"check"));
    let _ = std::fs::remove_file(&probe);
    result
        .err()
        .map(|error| format!("can't write the call log in {}: {error}", dir.display()))
}

/// The arguments a client starts the server with: everything the check
/// was given but `--check`, with full paths.
fn server_args(folder: &Path, options: &Options) -> Vec<String> {
    let mut args = vec!["--mcp".to_owned(), folder.display().to_string()];
    if options.profile != Profile::Standard {
        args.extend(["--profile".to_owned(), options.profile.name().to_owned()]);
    }
    if options.mode != axcenity_mcp::Mode::Edit {
        args.extend(["--mode".to_owned(), options.mode.name().to_owned()]);
    }
    for import in &options.import_folders {
        let full = std::fs::canonicalize(import).unwrap_or_else(|_| import.clone());
        args.extend(["--allow-import".to_owned(), full.display().to_string()]);
    }
    args
}

/// A JSON string: quotes and backslashes escaped (TOML basic strings take
/// the same escapes).
fn quote(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| format!("\"{text}\""))
}

/// For a shell: quoted only when it needs to be.
fn shell(text: &str) -> String {
    if text
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "-_./:\\".contains(c))
    {
        text.to_owned()
    } else {
        format!("\"{}\"", text.replace('"', "\\\""))
    }
}

/// How to add the server to Claude Code, JSON-config clients, VS Code and
/// Codex, with `exe` and the arguments filled in.
fn client_lines(exe: &Path, folder: &Path, options: &Options) -> String {
    let exe = exe.display().to_string();
    let args = server_args(folder, options);
    let json_args = args
        .iter()
        .map(|arg| quote(arg))
        .collect::<Vec<_>>()
        .join(", ");
    let shell_args = args
        .iter()
        .map(|arg| shell(arg))
        .collect::<Vec<_>>()
        .join(" ");
    let command = quote(&exe);
    [
        "Add it to a client (details: docs/mcp/README.md):".to_owned(),
        String::new(),
        "Claude Code:".to_owned(),
        format!("  claude mcp add axcenity -- {} {shell_args}", shell(&exe)),
        String::new(),
        "Google Antigravity, Claude Desktop, Cursor, Windsurf, Cline, LM Studio, Gemini CLI (\"mcpServers\" JSON):"
            .to_owned(),
        format!(
            "  {{\"mcpServers\": {{\"axcenity\": {{\"command\": {command}, \"args\": [{json_args}]}}}}}}"
        ),
        String::new(),
        "VS Code (.vscode/mcp.json):".to_owned(),
        format!(
            "  {{\"servers\": {{\"axcenity\": {{\"type\": \"stdio\", \"command\": {command}, \"args\": [{json_args}]}}}}}}"
        ),
        String::new(),
        "Codex (~/.codex/config.toml):".to_owned(),
        "  [mcp_servers.axcenity]".to_owned(),
        format!("  command = {command}"),
        format!("  args = [{json_args}]"),
    ]
    .join("\n")
}

/// Which AI client guide is shown in Settings ▸ AI connections.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum AiClientGuide {
    #[default]
    ClaudeCode,
    Antigravity,
    Codex,
    Cursor,
    ClaudeDesktop,
    VsCode,
    GeminiCli,
    LocalModels,
}

impl AiClientGuide {
    pub(crate) const ALL: [AiClientGuide; 8] = [
        AiClientGuide::ClaudeCode,
        AiClientGuide::Antigravity,
        AiClientGuide::Codex,
        AiClientGuide::Cursor,
        AiClientGuide::ClaudeDesktop,
        AiClientGuide::VsCode,
        AiClientGuide::GeminiCli,
        AiClientGuide::LocalModels,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            AiClientGuide::ClaudeCode => "Claude Code",
            AiClientGuide::Antigravity => "Google Antigravity",
            AiClientGuide::Codex => "ChatGPT / Codex",
            AiClientGuide::Cursor => "Cursor",
            AiClientGuide::ClaudeDesktop => "Claude Desktop",
            AiClientGuide::VsCode => "VS Code",
            AiClientGuide::GeminiCli => "Gemini CLI / Windsurf",
            AiClientGuide::LocalModels => "Local models",
        }
    }

    pub(crate) fn config_where(self) -> &'static str {
        match self {
            AiClientGuide::ClaudeCode => "Run in a terminal (or put the JSON in .mcp.json)",
            AiClientGuide::Antigravity => {
                "~/.gemini/antigravity/mcp_config.json (Manage MCP Servers → View raw config)"
            }
            AiClientGuide::Codex => "Run in a terminal, or add to ~/.codex/config.toml",
            AiClientGuide::Cursor => {
                ".cursor/mcp.json in the project, or ~/.cursor/mcp.json globally"
            }
            AiClientGuide::ClaudeDesktop => {
                "claude_desktop_config.json (%APPDATA%\\Claude\\ on Windows, ~/Library/Application Support/Claude/ on macOS, ~/.config/Claude/ on Linux)"
            }
            AiClientGuide::VsCode => ".vscode/mcp.json in your workspace (Copilot Agent mode)",
            AiClientGuide::GeminiCli => {
                "~/.gemini/settings.json (Gemini CLI) or Windsurf / Cline mcp_settings.json"
            }
            AiClientGuide::LocalModels => {
                "LM Studio mcp.json, Jan (Settings → MCP Servers), or ollmcp for Ollama"
            }
        }
    }

    pub(crate) fn steps(self) -> &'static [&'static str] {
        match self {
            AiClientGuide::ClaudeCode => &[
                "Turn on \"Allow AI connections\" above so Claude works directly in this window.",
                "Copy and run the terminal command below.",
                "Start Claude Code and ask it to inspect or build in your scene. Use /mcp__axcenity__axcenity_build for guided builds.",
            ],
            AiClientGuide::Antigravity => &[
                "Turn on \"Allow AI connections\" above so Antigravity edits live in this window.",
                "In Google Antigravity, open the MCP Servers panel → Manage MCP Servers → View raw config (mcp_config.json).",
                "Paste the JSON block below into mcpServers, save, and refresh/restart the MCP server.",
            ],
            AiClientGuide::Codex => &[
                "Turn on \"Allow AI connections\" above so Codex edits live in this window.",
                "Run the codex mcp add command below, or paste the TOML block into ~/.codex/config.toml.",
                "Note: ChatGPT web & OpenAI's Responses API only connect over HTTP, while this server speaks stdio — use OpenAI Codex CLI / IDE extension.",
            ],
            AiClientGuide::Cursor => &[
                "Turn on \"Allow AI connections\" above so Cursor edits live in this window.",
                "Create or open .cursor/mcp.json in your project (or ~/.cursor/mcp.json for all projects).",
                "Paste the JSON below, save, then enable \"axcenity\" in Cursor Settings → Tools & MCP.",
            ],
            AiClientGuide::ClaudeDesktop => &[
                "Turn on \"Allow AI connections\" above so Claude Desktop edits live in this window.",
                "Open claude_desktop_config.json (Developer → Edit Config inside Claude Desktop).",
                "Paste the JSON below into mcpServers, save, and fully restart Claude Desktop.",
            ],
            AiClientGuide::VsCode => &[
                "Turn on \"Allow AI connections\" above so Copilot edits live in this window.",
                "Create or open .vscode/mcp.json in your project workspace.",
                "Paste the JSON below, click Start above the server entry, and switch Copilot Chat to Agent mode.",
            ],
            AiClientGuide::GeminiCli => &[
                "Turn on \"Allow AI connections\" above so the client edits live in this window.",
                "Open ~/.gemini/settings.json (for Gemini CLI) or the MCP config file in Windsurf / Cline.",
                "Paste the JSON below into mcpServers, save, and restart the client.",
            ],
            AiClientGuide::LocalModels => &[
                "Pick the \"lite\" profile (13 tools, ~2.1k tokens) and set context length to at least 32k tokens.",
                "Use a model with native tool calling (e.g. Qwen 3 8B+, Qwen 2.5, gpt-oss-20b, Ministral) and enable tool use in LM Studio or Jan.",
                "Paste the JSON below into LM Studio's mcp.json (or enter Command + Arguments in Jan / ollmcp).",
            ],
        }
    }

    /// Formats the copyable command or configuration snippet for `folder` and `profile`.
    pub(crate) fn snippet(self, folder: &Path, profile: Profile) -> String {
        let exe_str = this_binary().display().to_string();
        let folder_str = folder.display().to_string();
        let mut args = vec!["--mcp".to_owned(), folder_str];
        if profile != Profile::Standard {
            args.extend(["--profile".to_owned(), profile.name().to_owned()]);
        }
        let json_args = args.iter().map(|a| quote(a)).collect::<Vec<_>>().join(", ");
        let shell_args = args.iter().map(|a| shell(a)).collect::<Vec<_>>().join(" ");
        let cmd_json = quote(&exe_str);
        match self {
            AiClientGuide::ClaudeCode => {
                format!(
                    "claude mcp add axcenity -- {} {shell_args}",
                    shell(&exe_str)
                )
            }
            AiClientGuide::Codex => format!(
                "# Terminal:\ncodex mcp add axcenity -- {} {shell_args}\n\n# Or ~/.codex/config.toml:\n[mcp_servers.axcenity]\ncommand = {cmd_json}\nargs = [{json_args}]\nstartup_timeout_sec = 30",
                shell(&exe_str)
            ),
            AiClientGuide::VsCode => format!(
                "{{\n  \"servers\": {{\n    \"axcenity\": {{\n      \"type\": \"stdio\",\n      \"command\": {cmd_json},\n      \"args\": [{json_args}]\n    }}\n  }}\n}}"
            ),
            AiClientGuide::Antigravity
            | AiClientGuide::Cursor
            | AiClientGuide::ClaudeDesktop
            | AiClientGuide::GeminiCli
            | AiClientGuide::LocalModels => format!(
                "{{\n  \"mcpServers\": {{\n    \"axcenity\": {{\n      \"command\": {cmd_json},\n      \"args\": [{json_args}]\n    }}\n  }}\n}}"
            ),
        }
    }
}

/// The line that adds the server for `folder` to Claude Code, with this
/// binary's path: for the Settings page and tests.
#[allow(dead_code)]
pub(crate) fn claude_code_command(folder: &Path) -> String {
    AiClientGuide::ClaudeCode.snippet(folder, Profile::Standard)
}

/// The binary clients should start: this one.
pub(super) fn this_binary() -> PathBuf {
    std::env::current_exe().unwrap_or_else(|_| PathBuf::from("axcenity-editor"))
}
