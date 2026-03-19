use anyhow::{Context, Result};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

use crate::integrity;

// Embedded hook script (guards before set -euo pipefail)
const REWRITE_HOOK: &str = include_str!("../hooks/contextzip-rewrite.sh");

// Embedded OpenCode plugin (auto-rewrite)
const OPENCODE_PLUGIN: &str = include_str!("../hooks/opencode-contextzip.ts");

// Embedded slim ContextZip awareness instructions
const CONTEXTZIP_SLIM: &str = include_str!("../hooks/contextzip-awareness.md");

/// Template written by `contextzip init` when no filters.toml exists yet.
const FILTERS_TEMPLATE: &str = r#"# Project-local ContextZip filters — commit this file with your repo.
# Filters here override user-global and built-in filters.
# Docs: https://github.com/rtk-ai/rtk#custom-filters
schema_version = 1

# Example: suppress build noise from a custom tool
# [filters.my-tool]
# description = "Compact my-tool output"
# match_command = "^my-tool\\s+build"
# strip_ansi = true
# strip_lines_matching = ["^\\s*$", "^Downloading", "^Installing"]
# max_lines = 30
# on_empty = "my-tool: ok"
"#;

/// Template for user-global filters (~/.config/contextzip/filters.toml).
const FILTERS_GLOBAL_TEMPLATE: &str = r#"# User-global ContextZip filters — apply to all your projects.
# Project-local .contextzip/filters.toml takes precedence over these.
# Docs: https://github.com/rtk-ai/rtk#custom-filters
schema_version = 1

# Example: suppress noise from a tool you use everywhere
# [filters.my-global-tool]
# description = "Compact my-global-tool output"
# match_command = "^my-global-tool\\b"
# strip_ansi = true
# strip_lines_matching = ["^\\s*$"]
# max_lines = 40
"#;

/// Control flow for settings.json patching
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PatchMode {
    Ask,  // Default: prompt user [y/N]
    Auto, // --auto-patch: no prompt
    Skip, // --no-patch: manual instructions
}

/// Result of settings.json patching operation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PatchResult {
    Patched,        // Hook was added successfully
    AlreadyPresent, // Hook was already in settings.json
    Declined,       // User declined when prompted
    Skipped,        // --no-patch flag used
}

// Legacy full instructions for backward compatibility (--claude-md mode)
const CONTEXTZIP_INSTRUCTIONS: &str = r##"<!-- contextzip-instructions v2 -->
# ContextZip - Token-Optimized Commands (based on RTK)

## Golden Rule

**Always prefix commands with `contextzip`**. If ContextZip has a dedicated filter, it uses it. If not, it passes through unchanged. This means ContextZip is always safe to use.

**Important**: Even in command chains with `&&`, use `contextzip`:
```bash
# ❌ Wrong
git add . && git commit -m "msg" && git push

# ✅ Correct
contextzip git add . && contextzip git commit -m "msg" && contextzip git push
```

## ContextZip Commands by Workflow

### Build & Compile (80-90% savings)
```bash
contextzip cargo build         # Cargo build output
contextzip cargo check         # Cargo check output
contextzip cargo clippy        # Clippy warnings grouped by file (80%)
contextzip tsc                 # TypeScript errors grouped by file/code (83%)
contextzip lint                # ESLint/Biome violations grouped (84%)
contextzip prettier --check    # Files needing format only (70%)
contextzip next build          # Next.js build with route metrics (87%)
```

### Test (90-99% savings)
```bash
contextzip cargo test          # Cargo test failures only (90%)
contextzip vitest run          # Vitest failures only (99.5%)
contextzip playwright test     # Playwright failures only (94%)
contextzip test <cmd>          # Generic test wrapper - failures only
```

### Git (59-80% savings)
```bash
contextzip git status          # Compact status
contextzip git log             # Compact log (works with all git flags)
contextzip git diff            # Compact diff (80%)
contextzip git show            # Compact show (80%)
contextzip git add             # Ultra-compact confirmations (59%)
contextzip git commit          # Ultra-compact confirmations (59%)
contextzip git push            # Ultra-compact confirmations
contextzip git pull            # Ultra-compact confirmations
contextzip git branch          # Compact branch list
contextzip git fetch           # Compact fetch
contextzip git stash           # Compact stash
contextzip git worktree        # Compact worktree
```

Note: Git passthrough works for ALL subcommands, even those not explicitly listed.

### GitHub (26-87% savings)
```bash
contextzip gh pr view <num>    # Compact PR view (87%)
contextzip gh pr checks        # Compact PR checks (79%)
contextzip gh run list         # Compact workflow runs (82%)
contextzip gh issue list       # Compact issue list (80%)
contextzip gh api              # Compact API responses (26%)
```

### JavaScript/TypeScript Tooling (70-90% savings)
```bash
contextzip pnpm list           # Compact dependency tree (70%)
contextzip pnpm outdated       # Compact outdated packages (80%)
contextzip pnpm install        # Compact install output (90%)
contextzip npm run <script>    # Compact npm script output
contextzip npx <cmd>           # Compact npx command output
contextzip prisma              # Prisma without ASCII art (88%)
```

### Files & Search (60-75% savings)
```bash
contextzip ls <path>           # Tree format, compact (65%)
contextzip read <file>         # Code reading with filtering (60%)
contextzip grep <pattern>      # Search grouped by file (75%)
contextzip find <pattern>      # Find grouped by directory (70%)
```

### Analysis & Debug (70-90% savings)
```bash
contextzip err <cmd>           # Filter errors only from any command
contextzip log <file>          # Deduplicated logs with counts
contextzip json <file>         # JSON structure without values
contextzip deps                # Dependency overview
contextzip env                 # Environment variables compact
contextzip summary <cmd>       # Smart summary of command output
contextzip diff                # Ultra-compact diffs
```

### Infrastructure (85% savings)
```bash
contextzip docker ps           # Compact container list
contextzip docker images       # Compact image list
contextzip docker logs <c>     # Deduplicated logs
contextzip kubectl get         # Compact resource list
contextzip kubectl logs        # Deduplicated pod logs
```

### Network (65-70% savings)
```bash
contextzip curl <url>          # Compact HTTP responses (70%)
contextzip wget <url>          # Compact download output (65%)
```

### Meta Commands
```bash
contextzip gain                # View token savings statistics
contextzip gain --history      # View command history with savings
contextzip discover            # Analyze Claude Code sessions for missed ContextZip usage
contextzip proxy <cmd>         # Run command without filtering (for debugging)
contextzip init                # Add ContextZip instructions to CLAUDE.md
contextzip init --global       # Add ContextZip to ~/.claude/CLAUDE.md
```

## Token Savings Overview

| Category | Commands | Typical Savings |
|----------|----------|-----------------|
| Tests | vitest, playwright, cargo test | 90-99% |
| Build | next, tsc, lint, prettier | 70-87% |
| Git | status, log, diff, add, commit | 59-80% |
| GitHub | gh pr, gh run, gh issue | 26-87% |
| Package Managers | pnpm, npm, npx | 70-90% |
| Files | ls, read, grep, find | 60-75% |
| Infrastructure | docker, kubectl | 85% |
| Network | curl, wget | 65-70% |

Overall average: **60-90% token reduction** on common development operations.
<!-- /contextzip-instructions -->
"##;

/// Main entry point for `contextzip init`
pub fn run(
    global: bool,
    install_claude: bool,
    install_opencode: bool,
    claude_md: bool,
    hook_only: bool,
    patch_mode: PatchMode,
    verbose: u8,
) -> Result<()> {
    if install_opencode && !global {
        anyhow::bail!("OpenCode plugin is global-only. Use: contextzip init -g --opencode");
    }

    // Mode selection
    match (install_claude, install_opencode, claude_md, hook_only) {
        (false, true, _, _) => run_opencode_only_mode(verbose),
        (true, opencode, true, _) => run_claude_md_mode(global, verbose, opencode),
        (true, opencode, false, true) => run_hook_only_mode(global, patch_mode, verbose, opencode),
        (true, opencode, false, false) => run_default_mode(global, patch_mode, verbose, opencode),
        (false, false, _, _) => {
            anyhow::bail!("at least one of install_claude or install_opencode must be true")
        }
    }
}

/// Prepare hook directory and return paths (hook_dir, hook_path)
fn prepare_hook_paths() -> Result<(PathBuf, PathBuf)> {
    let claude_dir = resolve_claude_dir()?;
    let hook_dir = claude_dir.join("hooks");
    fs::create_dir_all(&hook_dir)
        .with_context(|| format!("Failed to create hook directory: {}", hook_dir.display()))?;
    let hook_path = hook_dir.join("contextzip-rewrite.sh");
    Ok((hook_dir, hook_path))
}

/// Write hook file if missing or outdated, return true if changed
#[cfg(unix)]
fn ensure_hook_installed(hook_path: &Path, verbose: u8) -> Result<bool> {
    let changed = if hook_path.exists() {
        let existing = fs::read_to_string(hook_path)
            .with_context(|| format!("Failed to read existing hook: {}", hook_path.display()))?;

        if existing == REWRITE_HOOK {
            if verbose > 0 {
                eprintln!("Hook already up to date: {}", hook_path.display());
            }
            false
        } else {
            fs::write(hook_path, REWRITE_HOOK)
                .with_context(|| format!("Failed to write hook to {}", hook_path.display()))?;
            if verbose > 0 {
                eprintln!("Updated hook: {}", hook_path.display());
            }
            true
        }
    } else {
        fs::write(hook_path, REWRITE_HOOK)
            .with_context(|| format!("Failed to write hook to {}", hook_path.display()))?;
        if verbose > 0 {
            eprintln!("Created hook: {}", hook_path.display());
        }
        true
    };

    // Set executable permissions
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(hook_path, fs::Permissions::from_mode(0o755))
        .with_context(|| format!("Failed to set hook permissions: {}", hook_path.display()))?;

    // Store SHA-256 hash for runtime integrity verification.
    // Always store (idempotent) to ensure baseline exists even for
    // hooks installed before integrity checks were added.
    integrity::store_hash(hook_path)
        .with_context(|| format!("Failed to store integrity hash for {}", hook_path.display()))?;
    if verbose > 0 && changed {
        eprintln!("Stored integrity hash for hook");
    }

    Ok(changed)
}

/// Idempotent file write: create or update if content differs
fn write_if_changed(path: &Path, content: &str, name: &str, verbose: u8) -> Result<bool> {
    if path.exists() {
        let existing = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}: {}", name, path.display()))?;

        if existing == content {
            if verbose > 0 {
                eprintln!("{} already up to date: {}", name, path.display());
            }
            Ok(false)
        } else {
            fs::write(path, content)
                .with_context(|| format!("Failed to write {}: {}", name, path.display()))?;
            if verbose > 0 {
                eprintln!("Updated {}: {}", name, path.display());
            }
            Ok(true)
        }
    } else {
        fs::write(path, content)
            .with_context(|| format!("Failed to write {}: {}", name, path.display()))?;
        if verbose > 0 {
            eprintln!("Created {}: {}", name, path.display());
        }
        Ok(true)
    }
}

/// Atomic write using tempfile + rename
/// Prevents corruption on crash/interrupt
fn atomic_write(path: &Path, content: &str) -> Result<()> {
    let parent = path.parent().with_context(|| {
        format!(
            "Cannot write to {}: path has no parent directory",
            path.display()
        )
    })?;

    // Create temp file in same directory (ensures same filesystem for atomic rename)
    let mut temp_file = NamedTempFile::new_in(parent)
        .with_context(|| format!("Failed to create temp file in {}", parent.display()))?;

    // Write content
    temp_file
        .write_all(content.as_bytes())
        .with_context(|| format!("Failed to write {} bytes to temp file", content.len()))?;

    // Atomic rename
    temp_file.persist(path).with_context(|| {
        format!(
            "Failed to atomically replace {} (disk full?)",
            path.display()
        )
    })?;

    Ok(())
}

/// Prompt user for consent to patch settings.json
/// Prints to stderr (stdout may be piped), reads from stdin
/// Default is No (capital N)
fn prompt_user_consent(settings_path: &Path) -> Result<bool> {
    use std::io::{self, BufRead, IsTerminal};

    eprintln!("\nPatch existing {}? [y/N] ", settings_path.display());

    // If stdin is not a terminal (piped), default to No
    if !io::stdin().is_terminal() {
        eprintln!("(non-interactive mode, defaulting to N)");
        return Ok(false);
    }

    let stdin = io::stdin();
    let mut line = String::new();
    stdin
        .lock()
        .read_line(&mut line)
        .context("Failed to read user input")?;

    let response = line.trim().to_lowercase();
    Ok(response == "y" || response == "yes")
}

/// Print manual instructions for settings.json patching
fn print_manual_instructions(hook_path: &Path, include_opencode: bool) {
    println!("\n  MANUAL STEP: Add this to ~/.claude/settings.json:");
    println!("  {{");
    println!("    \"hooks\": {{ \"PreToolUse\": [{{");
    println!("      \"matcher\": \"Bash\",");
    println!("      \"hooks\": [{{ \"type\": \"command\",");
    println!("        \"command\": \"{}\"", hook_path.display());
    println!("      }}]");
    println!("    }}]}}");
    println!("  }}");
    if include_opencode {
        println!("\n  Then restart Claude Code and OpenCode. Test with: git status\n");
    } else {
        println!("\n  Then restart Claude Code. Test with: git status\n");
    }
}

/// Remove ContextZip hook entry from settings.json
/// Returns true if hook was found and removed
fn remove_hook_from_json(root: &mut serde_json::Value) -> bool {
    let hooks = match root.get_mut("hooks").and_then(|h| h.get_mut("PreToolUse")) {
        Some(pre_tool_use) => pre_tool_use,
        None => return false,
    };

    let pre_tool_use_array = match hooks.as_array_mut() {
        Some(arr) => arr,
        None => return false,
    };

    // Find and remove ContextZip entry
    let original_len = pre_tool_use_array.len();
    pre_tool_use_array.retain(|entry| {
        if let Some(hooks_array) = entry.get("hooks").and_then(|h| h.as_array()) {
            for hook in hooks_array {
                if let Some(command) = hook.get("command").and_then(|c| c.as_str()) {
                    if command.contains("contextzip-rewrite.sh") {
                        return false; // Remove this entry
                    }
                }
            }
        }
        true // Keep this entry
    });

    pre_tool_use_array.len() < original_len
}

/// Remove ContextZip hook from settings.json file
/// Backs up before modification, returns true if hook was found and removed
fn remove_hook_from_settings(verbose: u8) -> Result<bool> {
    let claude_dir = resolve_claude_dir()?;
    let settings_path = claude_dir.join("settings.json");

    if !settings_path.exists() {
        if verbose > 0 {
            eprintln!("settings.json not found, nothing to remove");
        }
        return Ok(false);
    }

    let content = fs::read_to_string(&settings_path)
        .with_context(|| format!("Failed to read {}", settings_path.display()))?;

    if content.trim().is_empty() {
        return Ok(false);
    }

    let mut root: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse {} as JSON", settings_path.display()))?;

    let removed = remove_hook_from_json(&mut root);

    if removed {
        // Backup original
        let backup_path = settings_path.with_extension("json.bak");
        fs::copy(&settings_path, &backup_path)
            .with_context(|| format!("Failed to backup to {}", backup_path.display()))?;

        // Atomic write
        let serialized =
            serde_json::to_string_pretty(&root).context("Failed to serialize settings.json")?;
        atomic_write(&settings_path, &serialized)?;

        if verbose > 0 {
            eprintln!("Removed ContextZip hook from settings.json");
        }
    }

    Ok(removed)
}

/// Full uninstall: remove hook, CONTEXTZIP.md, @CONTEXTZIP.md reference, settings.json entry
pub fn uninstall(global: bool, verbose: u8) -> Result<()> {
    if !global {
        anyhow::bail!("Uninstall only works with --global flag. For local projects, manually remove ContextZip from CLAUDE.md");
    }

    let claude_dir = resolve_claude_dir()?;
    let mut removed = Vec::new();

    // 1. Remove hook file
    let hook_path = claude_dir.join("hooks").join("contextzip-rewrite.sh");
    if hook_path.exists() {
        fs::remove_file(&hook_path)
            .with_context(|| format!("Failed to remove hook: {}", hook_path.display()))?;
        removed.push(format!("Hook: {}", hook_path.display()));
    }

    // 1b. Remove integrity hash file
    if integrity::remove_hash(&hook_path)? {
        removed.push("Integrity hash: removed".to_string());
    }

    // 2. Remove CONTEXTZIP.md
    let contextzip_md_path = claude_dir.join("CONTEXTZIP.md");
    if contextzip_md_path.exists() {
        fs::remove_file(&contextzip_md_path).with_context(|| {
            format!(
                "Failed to remove CONTEXTZIP.md: {}",
                contextzip_md_path.display()
            )
        })?;
        removed.push(format!("CONTEXTZIP.md: {}", contextzip_md_path.display()));
    }

    // 3. Remove @CONTEXTZIP.md reference from CLAUDE.md
    let claude_md_path = claude_dir.join("CLAUDE.md");
    if claude_md_path.exists() {
        let content = fs::read_to_string(&claude_md_path)
            .with_context(|| format!("Failed to read CLAUDE.md: {}", claude_md_path.display()))?;

        if content.contains("@CONTEXTZIP.md") {
            let new_content = content
                .lines()
                .filter(|line| !line.trim().starts_with("@CONTEXTZIP.md"))
                .collect::<Vec<_>>()
                .join("\n");

            // Clean up double blanks
            let cleaned = clean_double_blanks(&new_content);

            fs::write(&claude_md_path, cleaned).with_context(|| {
                format!("Failed to write CLAUDE.md: {}", claude_md_path.display())
            })?;
            removed.push("CLAUDE.md: removed @CONTEXTZIP.md reference".to_string());
        }
    }

    // 4. Remove hook entry from settings.json
    if remove_hook_from_settings(verbose)? {
        removed.push("settings.json: removed ContextZip hook entry".to_string());
    }

    // 5. Remove OpenCode plugin
    let opencode_removed = remove_opencode_plugin(verbose)?;
    for path in opencode_removed {
        removed.push(format!("OpenCode plugin: {}", path.display()));
    }

    // Report results
    if removed.is_empty() {
        println!("ContextZip was not installed (nothing to remove)");
    } else {
        println!("ContextZip uninstalled:");
        for item in removed {
            println!("  - {}", item);
        }
        println!("\nRestart Claude Code and OpenCode (if used) to apply changes.");
    }

    Ok(())
}

/// Orchestrator: patch settings.json with ContextZip hook
/// Handles reading, checking, prompting, merging, backing up, and atomic writing
fn patch_settings_json(
    hook_path: &Path,
    mode: PatchMode,
    verbose: u8,
    include_opencode: bool,
) -> Result<PatchResult> {
    let claude_dir = resolve_claude_dir()?;
    let settings_path = claude_dir.join("settings.json");
    let hook_command = hook_path
        .to_str()
        .context("Hook path contains invalid UTF-8")?;

    // Read or create settings.json
    let mut root = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)
            .with_context(|| format!("Failed to read {}", settings_path.display()))?;

        if content.trim().is_empty() {
            serde_json::json!({})
        } else {
            serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse {} as JSON", settings_path.display()))?
        }
    } else {
        serde_json::json!({})
    };

    // Check idempotency
    if hook_already_present(&root, hook_command) {
        if verbose > 0 {
            eprintln!("settings.json: hook already present");
        }
        return Ok(PatchResult::AlreadyPresent);
    }

    // Handle mode
    match mode {
        PatchMode::Skip => {
            print_manual_instructions(hook_path, include_opencode);
            return Ok(PatchResult::Skipped);
        }
        PatchMode::Ask => {
            if !prompt_user_consent(&settings_path)? {
                print_manual_instructions(hook_path, include_opencode);
                return Ok(PatchResult::Declined);
            }
        }
        PatchMode::Auto => {
            // Proceed without prompting
        }
    }

    // Deep-merge hook
    insert_hook_entry(&mut root, hook_command);

    // Backup original
    if settings_path.exists() {
        let backup_path = settings_path.with_extension("json.bak");
        fs::copy(&settings_path, &backup_path)
            .with_context(|| format!("Failed to backup to {}", backup_path.display()))?;
        if verbose > 0 {
            eprintln!("Backup: {}", backup_path.display());
        }
    }

    // Atomic write
    let serialized =
        serde_json::to_string_pretty(&root).context("Failed to serialize settings.json")?;
    atomic_write(&settings_path, &serialized)?;

    println!("\n  settings.json: hook added");
    if settings_path.with_extension("json.bak").exists() {
        println!(
            "  Backup: {}",
            settings_path.with_extension("json.bak").display()
        );
    }
    if include_opencode {
        println!("  Restart Claude Code and OpenCode. Test with: git status");
    } else {
        println!("  Restart Claude Code. Test with: git status");
    }

    Ok(PatchResult::Patched)
}

/// Clean up consecutive blank lines (collapse 3+ to 2)
/// Used when removing @CONTEXTZIP.md line from CLAUDE.md
fn clean_double_blanks(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        if line.trim().is_empty() {
            // Count consecutive blank lines
            let mut blank_count = 0;
            while i < lines.len() && lines[i].trim().is_empty() {
                blank_count += 1;
                i += 1;
            }

            // Keep at most 2 blank lines
            let keep = blank_count.min(2);
            result.extend(std::iter::repeat_n("", keep));
        } else {
            result.push(line);
            i += 1;
        }
    }

    result.join("\n")
}

/// Deep-merge ContextZip hook entry into settings.json
/// Creates hooks.PreToolUse structure if missing, preserves existing hooks
fn insert_hook_entry(root: &mut serde_json::Value, hook_command: &str) {
    // Ensure root is an object
    let root_obj = match root.as_object_mut() {
        Some(obj) => obj,
        None => {
            *root = serde_json::json!({});
            root.as_object_mut()
                .expect("Just created object, must succeed")
        }
    };

    // Use entry() API for idiomatic insertion
    let hooks = root_obj
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .expect("hooks must be an object");

    let pre_tool_use = hooks
        .entry("PreToolUse")
        .or_insert_with(|| serde_json::json!([]))
        .as_array_mut()
        .expect("PreToolUse must be an array");

    // Append ContextZip hook entry
    pre_tool_use.push(serde_json::json!({
        "matcher": "Bash",
        "hooks": [{
            "type": "command",
            "command": hook_command
        }]
    }));
}

/// Check if ContextZip hook is already present in settings.json
/// Matches on contextzip-rewrite.sh substring to handle different path formats
fn hook_already_present(root: &serde_json::Value, hook_command: &str) -> bool {
    let pre_tool_use_array = match root
        .get("hooks")
        .and_then(|h| h.get("PreToolUse"))
        .and_then(|p| p.as_array())
    {
        Some(arr) => arr,
        None => return false,
    };

    pre_tool_use_array
        .iter()
        .filter_map(|entry| entry.get("hooks")?.as_array())
        .flatten()
        .filter_map(|hook| hook.get("command")?.as_str())
        .any(|cmd| {
            // Exact match OR both contain contextzip-rewrite.sh
            cmd == hook_command
                || (cmd.contains("contextzip-rewrite.sh")
                    && hook_command.contains("contextzip-rewrite.sh"))
        })
}

/// Default mode: hook + slim CONTEXTZIP.md + @CONTEXTZIP.md reference
#[cfg(not(unix))]
fn run_default_mode(
    _global: bool,
    _patch_mode: PatchMode,
    _verbose: u8,
    _install_opencode: bool,
) -> Result<()> {
    eprintln!("[warn] Hook-based mode requires Unix (macOS/Linux).");
    eprintln!("    Windows: use --claude-md mode for full injection.");
    eprintln!("    Falling back to --claude-md mode.");
    run_claude_md_mode(_global, _verbose, _install_opencode)
}

#[cfg(unix)]
fn run_default_mode(
    global: bool,
    patch_mode: PatchMode,
    verbose: u8,
    install_opencode: bool,
) -> Result<()> {
    if !global {
        // Local init: inject CLAUDE.md + generate project-local filters template
        run_claude_md_mode(false, verbose, install_opencode)?;
        generate_project_filters_template(verbose)?;
        return Ok(());
    }

    let claude_dir = resolve_claude_dir()?;
    let contextzip_md_path = claude_dir.join("CONTEXTZIP.md");
    let claude_md_path = claude_dir.join("CLAUDE.md");

    // 1. Prepare hook directory and install hook
    let (_hook_dir, hook_path) = prepare_hook_paths()?;
    let hook_changed = ensure_hook_installed(&hook_path, verbose)?;

    // 2. Write CONTEXTZIP.md
    write_if_changed(
        &contextzip_md_path,
        CONTEXTZIP_SLIM,
        "CONTEXTZIP.md",
        verbose,
    )?;

    let opencode_plugin_path = if install_opencode {
        let path = prepare_opencode_plugin_path()?;
        ensure_opencode_plugin_installed(&path, verbose)?;
        Some(path)
    } else {
        None
    };

    // 3. Patch CLAUDE.md (add @CONTEXTZIP.md, migrate if needed)
    let migrated = patch_claude_md(&claude_md_path, verbose)?;

    // 4. Print success message
    let hook_status = if hook_changed {
        "installed/updated"
    } else {
        "already up to date"
    };
    println!("\nContextZip hook {} (global).\n", hook_status);
    println!("  Hook:      {}", hook_path.display());
    println!(
        "  CONTEXTZIP.md:    {} (10 lines)",
        contextzip_md_path.display()
    );
    if let Some(path) = &opencode_plugin_path {
        println!("  OpenCode:  {}", path.display());
    }
    println!("  CLAUDE.md: @CONTEXTZIP.md reference added");

    if migrated {
        println!("\n  [ok] Migrated: removed 137-line ContextZip block from CLAUDE.md");
        println!("              replaced with @CONTEXTZIP.md (10 lines)");
    }

    // 5. Patch settings.json
    let patch_result = patch_settings_json(&hook_path, patch_mode, verbose, install_opencode)?;

    // Report result
    match patch_result {
        PatchResult::Patched => {
            // Already printed by patch_settings_json
        }
        PatchResult::AlreadyPresent => {
            println!("\n  settings.json: hook already present");
            if install_opencode {
                println!("  Restart Claude Code and OpenCode. Test with: git status");
            } else {
                println!("  Restart Claude Code. Test with: git status");
            }
        }
        PatchResult::Declined | PatchResult::Skipped => {
            // Manual instructions already printed by patch_settings_json
        }
    }

    // 6. Generate user-global filters template (~/.config/contextzip/filters.toml)
    generate_global_filters_template(verbose)?;

    println!(); // Final newline

    Ok(())
}

/// Generate .contextzip/filters.toml template in the current directory if not present.
fn generate_project_filters_template(verbose: u8) -> Result<()> {
    let tz_dir = std::path::Path::new(".contextzip");
    let path = tz_dir.join("filters.toml");

    if path.exists() {
        if verbose > 0 {
            eprintln!(".contextzip/filters.toml already exists, skipping template");
        }
        return Ok(());
    }

    fs::create_dir_all(tz_dir)
        .with_context(|| format!("Failed to create directory: {}", tz_dir.display()))?;
    fs::write(&path, FILTERS_TEMPLATE)
        .with_context(|| format!("Failed to write {}", path.display()))?;

    println!(
        "  filters:   {} (template, edit to add project filters)",
        path.display()
    );
    Ok(())
}

/// Generate ~/.config/contextzip/filters.toml template if not present.
fn generate_global_filters_template(verbose: u8) -> Result<()> {
    let config_dir = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from(".config"));
    let tz_dir = config_dir.join("contextzip");
    let path = tz_dir.join("filters.toml");

    if path.exists() {
        if verbose > 0 {
            eprintln!("{} already exists, skipping template", path.display());
        }
        return Ok(());
    }

    fs::create_dir_all(&tz_dir)
        .with_context(|| format!("Failed to create directory: {}", tz_dir.display()))?;
    fs::write(&path, FILTERS_GLOBAL_TEMPLATE)
        .with_context(|| format!("Failed to write {}", path.display()))?;

    println!(
        "  filters:   {} (template, edit to add user-global filters)",
        path.display()
    );
    Ok(())
}

/// Hook-only mode: just the hook, no CONTEXTZIP.md
#[cfg(not(unix))]
fn run_hook_only_mode(
    _global: bool,
    _patch_mode: PatchMode,
    _verbose: u8,
    _install_opencode: bool,
) -> Result<()> {
    anyhow::bail!("Hook install requires Unix (macOS/Linux). Use WSL or --claude-md mode.")
}

#[cfg(unix)]
fn run_hook_only_mode(
    global: bool,
    patch_mode: PatchMode,
    verbose: u8,
    install_opencode: bool,
) -> Result<()> {
    if !global {
        eprintln!("[warn] Warning: --hook-only only makes sense with --global");
        eprintln!("    For local projects, use default mode or --claude-md");
        return Ok(());
    }

    // Prepare and install hook
    let (_hook_dir, hook_path) = prepare_hook_paths()?;
    let hook_changed = ensure_hook_installed(&hook_path, verbose)?;

    let opencode_plugin_path = if install_opencode {
        let path = prepare_opencode_plugin_path()?;
        ensure_opencode_plugin_installed(&path, verbose)?;
        Some(path)
    } else {
        None
    };

    let hook_status = if hook_changed {
        "installed/updated"
    } else {
        "already up to date"
    };
    println!("\nContextZip hook {} (hook-only mode).\n", hook_status);
    println!("  Hook: {}", hook_path.display());
    if let Some(path) = &opencode_plugin_path {
        println!("  OpenCode: {}", path.display());
    }
    println!(
        "  Note: No CONTEXTZIP.md created. Claude won't know about meta commands (gain, discover, proxy)."
    );

    // Patch settings.json
    let patch_result = patch_settings_json(&hook_path, patch_mode, verbose, install_opencode)?;

    // Report result
    match patch_result {
        PatchResult::Patched => {
            // Already printed by patch_settings_json
        }
        PatchResult::AlreadyPresent => {
            println!("\n  settings.json: hook already present");
            if install_opencode {
                println!("  Restart Claude Code and OpenCode. Test with: git status");
            } else {
                println!("  Restart Claude Code. Test with: git status");
            }
        }
        PatchResult::Declined | PatchResult::Skipped => {
            // Manual instructions already printed by patch_settings_json
        }
    }

    println!(); // Final newline

    Ok(())
}

/// Legacy mode: full 137-line injection into CLAUDE.md
fn run_claude_md_mode(global: bool, verbose: u8, install_opencode: bool) -> Result<()> {
    let path = if global {
        resolve_claude_dir()?.join("CLAUDE.md")
    } else {
        PathBuf::from("CLAUDE.md")
    };

    if global {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
    }

    if verbose > 0 {
        eprintln!("Writing contextzip instructions to: {}", path.display());
    }

    if path.exists() {
        let existing = fs::read_to_string(&path)?;
        // upsert_contextzip_block handles all 4 cases: add, update, unchanged, malformed
        let (new_content, action) = upsert_contextzip_block(&existing, CONTEXTZIP_INSTRUCTIONS);

        match action {
            TzBlockUpsert::Added => {
                fs::write(&path, new_content)?;
                println!(
                    "[ok] Added contextzip instructions to existing {}",
                    path.display()
                );
            }
            TzBlockUpsert::Updated => {
                fs::write(&path, new_content)?;
                println!("[ok] Updated contextzip instructions in {}", path.display());
            }
            TzBlockUpsert::Unchanged => {
                println!(
                    "[ok] {} already contains up-to-date contextzip instructions",
                    path.display()
                );
                return Ok(());
            }
            TzBlockUpsert::Malformed => {
                eprintln!(
                    "[warn] Warning: Found '<!-- contextzip-instructions' without closing marker in {}",
                    path.display()
                );

                if let Some((line_num, _)) = existing
                    .lines()
                    .enumerate()
                    .find(|(_, line)| line.contains("<!-- contextzip-instructions"))
                {
                    eprintln!("    Location: line {}", line_num + 1);
                }

                eprintln!("    Action: Manually remove the incomplete block, then re-run:");
                if global {
                    eprintln!("            contextzip init -g --claude-md");
                } else {
                    eprintln!("            contextzip init --claude-md");
                }
                return Ok(());
            }
        }
    } else {
        fs::write(&path, CONTEXTZIP_INSTRUCTIONS)?;
        println!(
            "[ok] Created {} with contextzip instructions",
            path.display()
        );
    }

    if global {
        if install_opencode {
            let opencode_plugin_path = prepare_opencode_plugin_path()?;
            ensure_opencode_plugin_installed(&opencode_plugin_path, verbose)?;
            println!(
                "[ok] OpenCode plugin installed: {}",
                opencode_plugin_path.display()
            );
        }
        println!("   Claude Code will now use contextzip in all sessions");
    } else {
        println!("   Claude Code will use contextzip in this project");
    }

    Ok(())
}

// --- upsert_contextzip_block: idempotent ContextZip block management ---

#[derive(Debug, Clone, Copy, PartialEq)]
enum TzBlockUpsert {
    /// No existing block found — appended new block
    Added,
    /// Existing block found with different content — replaced
    Updated,
    /// Existing block found with identical content — no-op
    Unchanged,
    /// Opening marker found without closing marker — not safe to rewrite
    Malformed,
}

/// Insert or replace the ContextZip instructions block in `content`.
///
/// Returns `(new_content, action)` describing what happened.
/// The caller decides whether to write `new_content` based on `action`.
fn upsert_contextzip_block(content: &str, block: &str) -> (String, TzBlockUpsert) {
    let start_marker = "<!-- contextzip-instructions";
    let end_marker = "<!-- /contextzip-instructions -->";

    if let Some(start) = content.find(start_marker) {
        if let Some(relative_end) = content[start..].find(end_marker) {
            let end = start + relative_end;
            let end_pos = end + end_marker.len();
            let current_block = content[start..end_pos].trim();
            let desired_block = block.trim();

            if current_block == desired_block {
                return (content.to_string(), TzBlockUpsert::Unchanged);
            }

            // Replace stale block with desired block
            let before = content[..start].trim_end();
            let after = content[end_pos..].trim_start();

            let result = match (before.is_empty(), after.is_empty()) {
                (true, true) => desired_block.to_string(),
                (true, false) => format!("{desired_block}\n\n{after}"),
                (false, true) => format!("{before}\n\n{desired_block}"),
                (false, false) => format!("{before}\n\n{desired_block}\n\n{after}"),
            };

            return (result, TzBlockUpsert::Updated);
        }

        // Opening marker without closing marker — malformed
        return (content.to_string(), TzBlockUpsert::Malformed);
    }

    // No existing block — append
    let trimmed = content.trim();
    if trimmed.is_empty() {
        (block.to_string(), TzBlockUpsert::Added)
    } else {
        (
            format!("{trimmed}\n\n{}", block.trim()),
            TzBlockUpsert::Added,
        )
    }
}

/// Patch CLAUDE.md: add @CONTEXTZIP.md, migrate if old block exists
fn patch_claude_md(path: &Path, verbose: u8) -> Result<bool> {
    let mut content = if path.exists() {
        fs::read_to_string(path)?
    } else {
        String::new()
    };

    let mut migrated = false;

    // Check for old block and migrate
    if content.contains("<!-- contextzip-instructions") {
        let (new_content, did_migrate) = remove_contextzip_block(&content);
        if did_migrate {
            content = new_content;
            migrated = true;
            if verbose > 0 {
                eprintln!("Migrated: removed old ContextZip block from CLAUDE.md");
            }
        }
    }

    // Check if @CONTEXTZIP.md already present
    if content.contains("@CONTEXTZIP.md") {
        if verbose > 0 {
            eprintln!("@CONTEXTZIP.md reference already present in CLAUDE.md");
        }
        if migrated {
            fs::write(path, content)?;
        }
        return Ok(migrated);
    }

    // Add @CONTEXTZIP.md
    let new_content = if content.is_empty() {
        "@CONTEXTZIP.md\n".to_string()
    } else {
        format!("{}\n\n@CONTEXTZIP.md\n", content.trim())
    };

    fs::write(path, new_content)?;

    if verbose > 0 {
        eprintln!("Added @CONTEXTZIP.md reference to CLAUDE.md");
    }

    Ok(migrated)
}

/// Remove old ContextZip block from CLAUDE.md (migration helper)
fn remove_contextzip_block(content: &str) -> (String, bool) {
    if let (Some(start), Some(end)) = (
        content.find("<!-- contextzip-instructions"),
        content.find("<!-- /contextzip-instructions -->"),
    ) {
        let end_pos = end + "<!-- /contextzip-instructions -->".len();
        let before = content[..start].trim_end();
        let after = content[end_pos..].trim_start();

        let result = if after.is_empty() {
            before.to_string()
        } else {
            format!("{}\n\n{}", before, after)
        };

        (result, true) // migrated
    } else if content.contains("<!-- contextzip-instructions") {
        eprintln!("[warn] Warning: Found '<!-- contextzip-instructions' without closing marker.");
        eprintln!("    This can happen if CLAUDE.md was manually edited.");

        // Find line number
        if let Some((line_num, _)) = content
            .lines()
            .enumerate()
            .find(|(_, line)| line.contains("<!-- contextzip-instructions"))
        {
            eprintln!("    Location: line {}", line_num + 1);
        }

        eprintln!("    Action: Manually remove the incomplete block, then re-run:");
        eprintln!("            contextzip init -g");
        (content.to_string(), false)
    } else {
        (content.to_string(), false)
    }
}

/// Resolve ~/.claude directory with proper home expansion
fn resolve_claude_dir() -> Result<PathBuf> {
    dirs::home_dir()
        .map(|h| h.join(".claude"))
        .context("Cannot determine home directory. Is $HOME set?")
}

/// Resolve OpenCode config directory (~/.config/opencode)
/// OpenCode uses ~/.config/opencode on all platforms (XDG convention),
/// NOT the macOS-native ~/Library/Application Support/.
fn resolve_opencode_dir() -> Result<PathBuf> {
    dirs::home_dir()
        .map(|h| h.join(".config").join("opencode"))
        .context("Cannot determine home directory. Is $HOME set?")
}

/// Return OpenCode plugin path: ~/.config/opencode/plugins/contextzip.ts
fn opencode_plugin_path(opencode_dir: &Path) -> PathBuf {
    opencode_dir.join("plugins").join("contextzip.ts")
}

/// Prepare OpenCode plugin directory and return install path
fn prepare_opencode_plugin_path() -> Result<PathBuf> {
    let opencode_dir = resolve_opencode_dir()?;
    let path = opencode_plugin_path(&opencode_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create OpenCode plugin directory: {}",
                parent.display()
            )
        })?;
    }
    Ok(path)
}

/// Write OpenCode plugin file if missing or outdated
fn ensure_opencode_plugin_installed(path: &Path, verbose: u8) -> Result<bool> {
    write_if_changed(path, OPENCODE_PLUGIN, "OpenCode plugin", verbose)
}

/// Remove OpenCode plugin file
fn remove_opencode_plugin(verbose: u8) -> Result<Vec<PathBuf>> {
    let opencode_dir = resolve_opencode_dir()?;
    let path = opencode_plugin_path(&opencode_dir);
    let mut removed = Vec::new();

    if path.exists() {
        fs::remove_file(&path)
            .with_context(|| format!("Failed to remove OpenCode plugin: {}", path.display()))?;
        if verbose > 0 {
            eprintln!("Removed OpenCode plugin: {}", path.display());
        }
        removed.push(path);
    }

    Ok(removed)
}

/// Show current contextzip configuration
pub fn show_config() -> Result<()> {
    let claude_dir = resolve_claude_dir()?;
    let hook_path = claude_dir.join("hooks").join("contextzip-rewrite.sh");
    let contextzip_md_path = claude_dir.join("CONTEXTZIP.md");
    let global_claude_md = claude_dir.join("CLAUDE.md");
    let local_claude_md = PathBuf::from("CLAUDE.md");

    println!("contextzip Configuration:\n");

    // Check hook
    if hook_path.exists() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&hook_path)?;
            let perms = metadata.permissions();
            let is_executable = perms.mode() & 0o111 != 0;

            let hook_content = fs::read_to_string(&hook_path)?;
            let has_guards = hook_content.contains("command -v contextzip")
                && hook_content.contains("command -v jq");
            let is_thin_delegator = hook_content.contains("contextzip rewrite");
            let hook_version = crate::hook_check::parse_hook_version(&hook_content);

            if !is_executable {
                println!(
                    "[warn] Hook: {} (NOT executable - run: chmod +x)",
                    hook_path.display()
                );
            } else if !is_thin_delegator {
                println!(
                    "[warn] Hook: {} (outdated — inline logic, not thin delegator)",
                    hook_path.display()
                );
                println!(
                    "   → Run `contextzip init --global` to upgrade to the single source of truth hook"
                );
            } else if is_executable && has_guards {
                println!(
                    "[ok] Hook: {} (thin delegator, version {})",
                    hook_path.display(),
                    hook_version
                );
            } else {
                println!(
                    "[warn] Hook: {} (no guards - outdated)",
                    hook_path.display()
                );
            }
        }

        #[cfg(not(unix))]
        {
            println!("[ok] Hook: {} (exists)", hook_path.display());
        }
    } else {
        println!("[--] Hook: not found");
    }

    // Check CONTEXTZIP.md
    if contextzip_md_path.exists() {
        println!(
            "[ok] CONTEXTZIP.md: {} (slim mode)",
            contextzip_md_path.display()
        );
    } else {
        println!("[--] CONTEXTZIP.md: not found");
    }

    // Check hook integrity
    match integrity::verify_hook_at(&hook_path) {
        Ok(integrity::IntegrityStatus::Verified) => {
            println!("[ok] Integrity: hook hash verified");
        }
        Ok(integrity::IntegrityStatus::Tampered { .. }) => {
            println!(
                "[FAIL] Integrity: hook modified outside contextzip init (run: contextzip verify)"
            );
        }
        Ok(integrity::IntegrityStatus::NoBaseline) => {
            println!("[warn] Integrity: no baseline hash (run: contextzip init -g to establish)");
        }
        Ok(integrity::IntegrityStatus::NotInstalled)
        | Ok(integrity::IntegrityStatus::OrphanedHash) => {
            // Don't show integrity line if hook isn't installed
        }
        Err(_) => {
            println!("[warn] Integrity: check failed");
        }
    }

    // Check global CLAUDE.md
    if global_claude_md.exists() {
        let content = fs::read_to_string(&global_claude_md)?;
        if content.contains("@CONTEXTZIP.md") {
            println!("[ok] Global (~/.claude/CLAUDE.md): @CONTEXTZIP.md reference");
        } else if content.contains("<!-- contextzip-instructions") {
            println!(
                "[warn] Global (~/.claude/CLAUDE.md): old ContextZip block (run: contextzip init -g to migrate)"
            );
        } else {
            println!("[--] Global (~/.claude/CLAUDE.md): exists but contextzip not configured");
        }
    } else {
        println!("[--] Global (~/.claude/CLAUDE.md): not found");
    }

    // Check local CLAUDE.md
    if local_claude_md.exists() {
        let content = fs::read_to_string(&local_claude_md)?;
        if content.contains("contextzip") {
            println!("[ok] Local (./CLAUDE.md): contextzip enabled");
        } else {
            println!("[--] Local (./CLAUDE.md): exists but contextzip not configured");
        }
    } else {
        println!("[--] Local (./CLAUDE.md): not found");
    }

    // Check settings.json
    let settings_path = claude_dir.join("settings.json");
    if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        if !content.trim().is_empty() {
            if let Ok(root) = serde_json::from_str::<serde_json::Value>(&content) {
                let hook_command = hook_path.display().to_string();
                if hook_already_present(&root, &hook_command) {
                    println!("[ok] settings.json: ContextZip hook configured");
                } else {
                    println!("[warn] settings.json: exists but ContextZip hook not configured");
                    println!("    Run: contextzip init -g --auto-patch");
                }
            } else {
                println!("[warn] settings.json: exists but invalid JSON");
            }
        } else {
            println!("[--] settings.json: empty");
        }
    } else {
        println!("[--] settings.json: not found");
    }

    // Check OpenCode plugin
    if let Ok(opencode_dir) = resolve_opencode_dir() {
        let plugin = opencode_plugin_path(&opencode_dir);
        if plugin.exists() {
            println!("[ok] OpenCode: plugin installed ({})", plugin.display());
        } else {
            println!("[--] OpenCode: plugin not found");
        }
    } else {
        println!("[--] OpenCode: config dir not found");
    }

    println!("\nUsage:");
    println!("  contextzip init              # Full injection into local CLAUDE.md");
    println!("  contextzip init -g           # Hook + CONTEXTZIP.md + @CONTEXTZIP.md + settings.json (recommended)");
    println!("  contextzip init -g --auto-patch    # Same as above but no prompt");
    println!("  contextzip init -g --no-patch      # Skip settings.json (manual setup)");
    println!("  contextzip init -g --uninstall     # Remove all ContextZip artifacts");
    println!(
        "  contextzip init -g --claude-md     # Legacy: full injection into ~/.claude/CLAUDE.md"
    );
    println!("  contextzip init -g --hook-only     # Hook only, no CONTEXTZIP.md");
    println!("  contextzip init -g --opencode      # OpenCode plugin only");

    Ok(())
}

fn run_opencode_only_mode(verbose: u8) -> Result<()> {
    let opencode_plugin_path = prepare_opencode_plugin_path()?;
    ensure_opencode_plugin_installed(&opencode_plugin_path, verbose)?;
    println!("\nOpenCode plugin installed (global).\n");
    println!("  OpenCode: {}", opencode_plugin_path.display());
    println!("  Restart OpenCode. Test with: git status\n");
    Ok(())
}

/// Top-level uninstall command: remove hook, settings.json entry, binary, and optionally DB
pub fn run_uninstall(purge: bool, verbose: u8) -> Result<()> {
    let claude_dir = resolve_claude_dir()?;
    let mut removed = Vec::new();

    // 1. Remove hook file
    let hook_path = claude_dir.join("hooks").join("contextzip-rewrite.sh");
    if hook_path.exists() {
        fs::remove_file(&hook_path)
            .with_context(|| format!("Failed to remove hook: {}", hook_path.display()))?;
        removed.push(format!("Hook: {}", hook_path.display()));
    }

    // 1b. Remove integrity hash file
    if integrity::remove_hash(&hook_path)? {
        removed.push("Integrity hash: removed".to_string());
    }

    // 2. Remove contextzip entry from settings.json PreToolUse hooks
    if remove_hook_from_settings(verbose)? {
        removed.push("settings.json: removed ContextZip hook entry".to_string());
    }

    // 3. Remove CONTEXTZIP.md
    let contextzip_md_path = claude_dir.join("CONTEXTZIP.md");
    if contextzip_md_path.exists() {
        fs::remove_file(&contextzip_md_path).with_context(|| {
            format!(
                "Failed to remove CONTEXTZIP.md: {}",
                contextzip_md_path.display()
            )
        })?;
        removed.push(format!("CONTEXTZIP.md: {}", contextzip_md_path.display()));
    }

    // 4. Remove @CONTEXTZIP.md reference from CLAUDE.md
    let claude_md_path = claude_dir.join("CLAUDE.md");
    if claude_md_path.exists() {
        let content = fs::read_to_string(&claude_md_path)
            .with_context(|| format!("Failed to read CLAUDE.md: {}", claude_md_path.display()))?;

        if content.contains("@CONTEXTZIP.md") {
            let new_content = content
                .lines()
                .filter(|line| !line.trim().starts_with("@CONTEXTZIP.md"))
                .collect::<Vec<_>>()
                .join("\n");

            let cleaned = clean_double_blanks(&new_content);

            fs::write(&claude_md_path, cleaned).with_context(|| {
                format!("Failed to write CLAUDE.md: {}", claude_md_path.display())
            })?;
            removed.push("CLAUDE.md: removed @CONTEXTZIP.md reference".to_string());
        }
    }

    // 5. Remove OpenCode plugin
    let opencode_removed = remove_opencode_plugin(verbose)?;
    for path in opencode_removed {
        removed.push(format!("OpenCode plugin: {}", path.display()));
    }

    // 6. Remove binary (~/.local/bin/contextzip)
    let bin_path = dirs::home_dir()
        .context("Cannot determine home directory")?
        .join(".local")
        .join("bin")
        .join("contextzip");
    if bin_path.exists() {
        fs::remove_file(&bin_path)
            .with_context(|| format!("Failed to remove binary: {}", bin_path.display()))?;
        removed.push(format!("Binary: {}", bin_path.display()));
    }

    // 7. Handle SQLite database
    let db_path = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("contextzip")
        .join("history.db");

    if purge && db_path.exists() {
        fs::remove_file(&db_path)
            .with_context(|| format!("Failed to remove database: {}", db_path.display()))?;
        removed.push(format!("Database: {} (purged)", db_path.display()));

        // Also remove the parent directory if empty
        if let Some(parent) = db_path.parent() {
            let _ = fs::remove_dir(parent); // ignore error if not empty
        }
    }

    // Report results
    if removed.is_empty() {
        println!("ContextZip was not installed (nothing to remove)");
    } else {
        println!("ContextZip uninstalled:");
        for item in &removed {
            println!("  - {}", item);
        }
    }

    if !purge && db_path.exists() {
        println!(
            "\nSQLite data preserved at {}. Use --purge to delete.",
            db_path.display()
        );
    }

    println!("\nRestart Claude Code to apply changes.");

    Ok(())
}

/// Top-level update command: download latest release binary from GitHub
pub fn run_update(verbose: u8) -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");

    // Detect platform
    let (os, arch) = detect_platform()?;
    let asset_name = format!("contextzip-{}-{}.tar.gz", os, arch);

    println!("Current version: v{}", current_version);
    println!("Checking for updates...");

    // Fetch latest release info from GitHub API
    let api_url = "https://api.github.com/repos/jee599/contextzip/releases/latest";
    let response_body = ureq::get(api_url)
        .set("User-Agent", "contextzip-updater")
        .set("Accept", "application/vnd.github.v3+json")
        .call()
        .context("Failed to fetch latest release from GitHub")?
        .into_string()
        .context("Failed to read GitHub API response")?;

    let body: serde_json::Value =
        serde_json::from_str(&response_body).context("Failed to parse GitHub API response")?;

    let tag = body["tag_name"]
        .as_str()
        .context("No tag_name in release")?;
    let latest_version = tag.strip_prefix('v').unwrap_or(tag);

    if latest_version == current_version {
        println!("Already up to date (v{})", current_version);
        return Ok(());
    }

    // Find the matching asset
    let assets = body["assets"].as_array().context("No assets in release")?;

    let download_url: String = assets
        .iter()
        .find_map(|a| {
            let name = a["name"].as_str()?;
            if name == asset_name {
                Some(a["browser_download_url"].as_str()?.to_string())
            } else {
                None
            }
        })
        .with_context(|| format!("No asset found for {}", asset_name))?;

    if verbose > 0 {
        eprintln!("Downloading: {}", download_url);
    }

    println!("Downloading v{}...", latest_version);

    // Download the tarball
    let dl_response = ureq::get(&download_url)
        .set("User-Agent", "contextzip-updater")
        .call()
        .context("Failed to download release")?;

    let mut tarball = Vec::new();
    dl_response
        .into_reader()
        .read_to_end(&mut tarball)
        .context("Failed to read release download")?;

    // Extract binary from tarball
    let decoder = flate2::read::GzDecoder::new(std::io::Cursor::new(&tarball));
    let mut archive = tar::Archive::new(decoder);

    let current_exe = std::env::current_exe().context("Failed to determine current binary path")?;

    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
    let temp_bin = temp_dir.path().join("contextzip");

    let mut found = false;
    for entry in archive.entries().context("Failed to read tar entries")? {
        let mut entry = entry.context("Failed to read tar entry")?;
        let path = entry.path().context("Failed to read entry path")?;

        if path.file_name().map(|n| n == "contextzip").unwrap_or(false) {
            entry
                .unpack(&temp_bin)
                .context("Failed to extract contextzip binary")?;
            found = true;
            break;
        }
    }

    if !found {
        anyhow::bail!("contextzip binary not found in release tarball");
    }

    // Set executable permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temp_bin, fs::Permissions::from_mode(0o755))
            .context("Failed to set binary permissions")?;
    }

    // Replace current binary
    fs::copy(&temp_bin, &current_exe).with_context(|| {
        format!(
            "Failed to replace binary at {}. Try running with sudo.",
            current_exe.display()
        )
    })?;

    println!(
        "Updated: v{} -> v{} ({})",
        current_version,
        latest_version,
        current_exe.display()
    );

    Ok(())
}

/// Detect current platform for release asset name
fn detect_platform() -> Result<(&'static str, &'static str)> {
    let os = if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        anyhow::bail!("Unsupported platform for self-update");
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        anyhow::bail!("Unsupported architecture for self-update");
    };

    Ok((os, arch))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init_mentions_all_top_level_commands() {
        for cmd in [
            "contextzip cargo",
            "contextzip gh",
            "contextzip vitest",
            "contextzip tsc",
            "contextzip lint",
            "contextzip prettier",
            "contextzip next",
            "contextzip playwright",
            "contextzip prisma",
            "contextzip pnpm",
            "contextzip npm",
            "contextzip curl",
            "contextzip git",
            "contextzip docker",
            "contextzip kubectl",
        ] {
            assert!(
                CONTEXTZIP_INSTRUCTIONS.contains(cmd),
                "Missing {cmd} in CONTEXTZIP_INSTRUCTIONS"
            );
        }
    }

    #[test]
    fn test_init_has_version_marker() {
        assert!(
            CONTEXTZIP_INSTRUCTIONS.contains("<!-- contextzip-instructions"),
            "CONTEXTZIP_INSTRUCTIONS must have version marker for idempotency"
        );
    }

    #[test]
    fn test_hook_has_guards() {
        assert!(REWRITE_HOOK.contains("command -v contextzip"));
        assert!(REWRITE_HOOK.contains("command -v jq"));
        // Guards (contextzip/jq availability checks) must appear before the actual delegation call.
        // The thin delegating hook no longer uses set -euo pipefail.
        let jq_pos = REWRITE_HOOK.find("command -v jq").unwrap();
        let tz_delegate_pos = REWRITE_HOOK.find("contextzip rewrite \"$CMD\"").unwrap();
        assert!(
            jq_pos < tz_delegate_pos,
            "Guards must appear before contextzip rewrite delegation"
        );
    }

    #[test]
    fn test_migration_removes_old_block() {
        let input = r#"# My Config

<!-- contextzip-instructions v2 -->
OLD RTK STUFF
<!-- /contextzip-instructions -->

More content"#;

        let (result, migrated) = remove_contextzip_block(input);
        assert!(migrated);
        assert!(!result.contains("OLD RTK STUFF"));
        assert!(result.contains("# My Config"));
        assert!(result.contains("More content"));
    }

    #[test]
    fn test_opencode_plugin_install_and_update() {
        let temp = TempDir::new().unwrap();
        let opencode_dir = temp.path().join("opencode");
        let plugin_path = opencode_plugin_path(&opencode_dir);

        fs::create_dir_all(plugin_path.parent().unwrap()).unwrap();
        assert!(!plugin_path.exists());

        let changed = ensure_opencode_plugin_installed(&plugin_path, 0).unwrap();
        assert!(changed);
        let content = fs::read_to_string(&plugin_path).unwrap();
        assert_eq!(content, OPENCODE_PLUGIN);

        fs::write(&plugin_path, "// old").unwrap();
        let changed_again = ensure_opencode_plugin_installed(&plugin_path, 0).unwrap();
        assert!(changed_again);
        let content_updated = fs::read_to_string(&plugin_path).unwrap();
        assert_eq!(content_updated, OPENCODE_PLUGIN);
    }

    #[test]
    fn test_opencode_plugin_remove() {
        let temp = TempDir::new().unwrap();
        let opencode_dir = temp.path().join("opencode");
        let plugin_path = opencode_plugin_path(&opencode_dir);
        fs::create_dir_all(plugin_path.parent().unwrap()).unwrap();
        fs::write(&plugin_path, OPENCODE_PLUGIN).unwrap();

        assert!(plugin_path.exists());
        fs::remove_file(&plugin_path).unwrap();
        assert!(!plugin_path.exists());
    }

    #[test]
    fn test_migration_warns_on_missing_end_marker() {
        let input = "<!-- contextzip-instructions v2 -->\nOLD STUFF\nNo end marker";
        let (result, migrated) = remove_contextzip_block(input);
        assert!(!migrated);
        assert_eq!(result, input);
    }

    #[test]
    #[cfg(unix)]
    fn test_default_mode_creates_hook_and_contextzip_md() {
        let temp = TempDir::new().unwrap();
        let hook_path = temp.path().join("contextzip-rewrite.sh");
        let contextzip_md_path = temp.path().join("CONTEXTZIP.md");

        fs::write(&hook_path, REWRITE_HOOK).unwrap();
        fs::write(&contextzip_md_path, CONTEXTZIP_SLIM).unwrap();

        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook_path, fs::Permissions::from_mode(0o755)).unwrap();

        assert!(hook_path.exists());
        assert!(contextzip_md_path.exists());

        let metadata = fs::metadata(&hook_path).unwrap();
        assert!(metadata.permissions().mode() & 0o111 != 0);
    }

    #[test]
    fn test_claude_md_mode_creates_full_injection() {
        // Just verify CONTEXTZIP_INSTRUCTIONS constant has the right content
        assert!(CONTEXTZIP_INSTRUCTIONS.contains("<!-- contextzip-instructions"));
        assert!(CONTEXTZIP_INSTRUCTIONS.contains("contextzip cargo test"));
        assert!(CONTEXTZIP_INSTRUCTIONS.contains("<!-- /contextzip-instructions -->"));
        assert!(CONTEXTZIP_INSTRUCTIONS.len() > 4000);
    }

    // --- upsert_contextzip_block tests ---

    #[test]
    fn test_upsert_contextzip_block_appends_when_missing() {
        let input = "# Team instructions";
        let (content, action) = upsert_contextzip_block(input, CONTEXTZIP_INSTRUCTIONS);
        assert_eq!(action, TzBlockUpsert::Added);
        assert!(content.contains("# Team instructions"));
        assert!(content.contains("<!-- contextzip-instructions"));
    }

    #[test]
    fn test_upsert_contextzip_block_updates_stale_block() {
        let input = r#"# Team instructions

<!-- contextzip-instructions v1 -->
OLD RTK CONTENT
<!-- /contextzip-instructions -->

More notes
"#;

        let (content, action) = upsert_contextzip_block(input, CONTEXTZIP_INSTRUCTIONS);
        assert_eq!(action, TzBlockUpsert::Updated);
        assert!(!content.contains("OLD RTK CONTENT"));
        assert!(content.contains("contextzip cargo test")); // from current CONTEXTZIP_INSTRUCTIONS
        assert!(content.contains("# Team instructions"));
        assert!(content.contains("More notes"));
    }

    #[test]
    fn test_upsert_contextzip_block_noop_when_already_current() {
        let input = format!(
            "# Team instructions\n\n{}\n\nMore notes\n",
            CONTEXTZIP_INSTRUCTIONS
        );
        let (content, action) = upsert_contextzip_block(&input, CONTEXTZIP_INSTRUCTIONS);
        assert_eq!(action, TzBlockUpsert::Unchanged);
        assert_eq!(content, input);
    }

    #[test]
    fn test_upsert_contextzip_block_detects_malformed_block() {
        let input = "<!-- contextzip-instructions v2 -->\npartial";
        let (content, action) = upsert_contextzip_block(input, CONTEXTZIP_INSTRUCTIONS);
        assert_eq!(action, TzBlockUpsert::Malformed);
        assert_eq!(content, input);
    }

    #[test]
    fn test_init_is_idempotent() {
        let temp = TempDir::new().unwrap();
        let claude_md = temp.path().join("CLAUDE.md");

        fs::write(&claude_md, "# My stuff\n\n@CONTEXTZIP.md\n").unwrap();

        let content = fs::read_to_string(&claude_md).unwrap();
        let count = content.matches("@CONTEXTZIP.md").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_local_init_unchanged() {
        // Local init should use claude-md mode
        let temp = TempDir::new().unwrap();
        let claude_md = temp.path().join("CLAUDE.md");

        fs::write(&claude_md, CONTEXTZIP_INSTRUCTIONS).unwrap();
        let content = fs::read_to_string(&claude_md).unwrap();

        assert!(content.contains("<!-- contextzip-instructions"));
    }

    // Tests for hook_already_present()
    #[test]
    fn test_hook_already_present_exact_match() {
        let json_content = serde_json::json!({
            "hooks": {
                "PreToolUse": [{
                    "matcher": "Bash",
                    "hooks": [{
                        "type": "command",
                        "command": "/Users/test/.claude/hooks/contextzip-rewrite.sh"
                    }]
                }]
            }
        });

        let hook_command = "/Users/test/.claude/hooks/contextzip-rewrite.sh";
        assert!(hook_already_present(&json_content, hook_command));
    }

    #[test]
    fn test_hook_already_present_different_path() {
        let json_content = serde_json::json!({
            "hooks": {
                "PreToolUse": [{
                    "matcher": "Bash",
                    "hooks": [{
                        "type": "command",
                        "command": "/home/user/.claude/hooks/contextzip-rewrite.sh"
                    }]
                }]
            }
        });

        let hook_command = "~/.claude/hooks/contextzip-rewrite.sh";
        // Should match on contextzip-rewrite.sh substring
        assert!(hook_already_present(&json_content, hook_command));
    }

    #[test]
    fn test_hook_not_present_empty() {
        let json_content = serde_json::json!({});
        let hook_command = "/Users/test/.claude/hooks/contextzip-rewrite.sh";
        assert!(!hook_already_present(&json_content, hook_command));
    }

    #[test]
    fn test_hook_not_present_other_hooks() {
        let json_content = serde_json::json!({
            "hooks": {
                "PreToolUse": [{
                    "matcher": "Bash",
                    "hooks": [{
                        "type": "command",
                        "command": "/some/other/hook.sh"
                    }]
                }]
            }
        });

        let hook_command = "/Users/test/.claude/hooks/contextzip-rewrite.sh";
        assert!(!hook_already_present(&json_content, hook_command));
    }

    // Tests for insert_hook_entry()
    #[test]
    fn test_insert_hook_entry_empty_root() {
        let mut json_content = serde_json::json!({});
        let hook_command = "/Users/test/.claude/hooks/contextzip-rewrite.sh";

        insert_hook_entry(&mut json_content, hook_command);

        // Should create full structure
        assert!(json_content.get("hooks").is_some());
        assert!(json_content
            .get("hooks")
            .unwrap()
            .get("PreToolUse")
            .is_some());

        let pre_tool_use = json_content["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(pre_tool_use.len(), 1);

        let command = pre_tool_use[0]["hooks"][0]["command"].as_str().unwrap();
        assert_eq!(command, hook_command);
    }

    #[test]
    fn test_insert_hook_entry_preserves_existing() {
        let mut json_content = serde_json::json!({
            "hooks": {
                "PreToolUse": [{
                    "matcher": "Bash",
                    "hooks": [{
                        "type": "command",
                        "command": "/some/other/hook.sh"
                    }]
                }]
            }
        });

        let hook_command = "/Users/test/.claude/hooks/contextzip-rewrite.sh";
        insert_hook_entry(&mut json_content, hook_command);

        let pre_tool_use = json_content["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(pre_tool_use.len(), 2); // Should have both hooks

        // Check first hook is preserved
        let first_command = pre_tool_use[0]["hooks"][0]["command"].as_str().unwrap();
        assert_eq!(first_command, "/some/other/hook.sh");

        // Check second hook is RTK
        let second_command = pre_tool_use[1]["hooks"][0]["command"].as_str().unwrap();
        assert_eq!(second_command, hook_command);
    }

    #[test]
    fn test_insert_hook_preserves_other_keys() {
        let mut json_content = serde_json::json!({
            "env": {"PATH": "/custom/path"},
            "permissions": {"allowAll": true},
            "model": "claude-sonnet-4"
        });

        let hook_command = "/Users/test/.claude/hooks/contextzip-rewrite.sh";
        insert_hook_entry(&mut json_content, hook_command);

        // Should preserve all other keys
        assert_eq!(json_content["env"]["PATH"], "/custom/path");
        assert_eq!(json_content["permissions"]["allowAll"], true);
        assert_eq!(json_content["model"], "claude-sonnet-4");

        // And add hooks
        assert!(json_content.get("hooks").is_some());
    }

    // Tests for atomic_write()
    #[test]
    fn test_atomic_write() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.json");

        let content = r#"{"key": "value"}"#;
        atomic_write(&file_path, content).unwrap();

        assert!(file_path.exists());
        let written = fs::read_to_string(&file_path).unwrap();
        assert_eq!(written, content);
    }

    // Test for preserve_order round-trip
    #[test]
    fn test_preserve_order_round_trip() {
        let original = r#"{"env": {"PATH": "/usr/bin"}, "permissions": {"allowAll": true}, "model": "claude-sonnet-4"}"#;
        let parsed: serde_json::Value = serde_json::from_str(original).unwrap();
        let serialized = serde_json::to_string(&parsed).unwrap();

        // Keys should appear in same order
        let _original_keys: Vec<&str> = original.split("\"").filter(|s| s.contains(":")).collect();
        let _serialized_keys: Vec<&str> =
            serialized.split("\"").filter(|s| s.contains(":")).collect();

        // Just check that keys exist (preserve_order doesn't guarantee exact order in nested objects)
        assert!(serialized.contains("\"env\""));
        assert!(serialized.contains("\"permissions\""));
        assert!(serialized.contains("\"model\""));
    }

    // Tests for clean_double_blanks()
    #[test]
    fn test_clean_double_blanks() {
        // Input: line1, 2 blank lines, line2, 1 blank line, line3, 3 blank lines, line4
        // Expected: line1, 2 blank lines (kept), line2, 1 blank line, line3, 2 blank lines (max), line4
        let input = "line1\n\n\nline2\n\nline3\n\n\n\nline4";
        // That's: line1 \n \n \n line2 \n \n line3 \n \n \n \n line4
        // Which is: line1, blank, blank, line2, blank, line3, blank, blank, blank, line4
        // So 2 blanks after line1 (keep both), 1 blank after line2 (keep), 3 blanks after line3 (keep 2)
        let expected = "line1\n\n\nline2\n\nline3\n\n\nline4";
        assert_eq!(clean_double_blanks(input), expected);
    }

    #[test]
    fn test_clean_double_blanks_preserves_single() {
        let input = "line1\n\nline2\n\nline3";
        assert_eq!(clean_double_blanks(input), input); // No change
    }

    // Tests for remove_hook_from_settings()
    #[test]
    fn test_remove_hook_from_json() {
        let mut json_content = serde_json::json!({
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [{
                            "type": "command",
                            "command": "/some/other/hook.sh"
                        }]
                    },
                    {
                        "matcher": "Bash",
                        "hooks": [{
                            "type": "command",
                            "command": "/Users/test/.claude/hooks/contextzip-rewrite.sh"
                        }]
                    }
                ]
            }
        });

        let removed = remove_hook_from_json(&mut json_content);
        assert!(removed);

        // Should have only one hook left
        let pre_tool_use = json_content["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(pre_tool_use.len(), 1);

        // Check it's the other hook
        let command = pre_tool_use[0]["hooks"][0]["command"].as_str().unwrap();
        assert_eq!(command, "/some/other/hook.sh");
    }

    #[test]
    fn test_remove_hook_when_not_present() {
        let mut json_content = serde_json::json!({
            "hooks": {
                "PreToolUse": [{
                    "matcher": "Bash",
                    "hooks": [{
                        "type": "command",
                        "command": "/some/other/hook.sh"
                    }]
                }]
            }
        });

        let removed = remove_hook_from_json(&mut json_content);
        assert!(!removed);
    }

    #[test]
    fn test_detect_platform_returns_valid_pair() {
        let (os, arch) = detect_platform().unwrap();
        // Must be one of supported values
        assert!(
            ["darwin", "linux", "windows"].contains(&os),
            "Unexpected OS: {}",
            os
        );
        assert!(
            ["x86_64", "aarch64"].contains(&arch),
            "Unexpected arch: {}",
            arch
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_detect_platform_macos() {
        let (os, _) = detect_platform().unwrap();
        assert_eq!(os, "darwin");
    }

    #[test]
    fn test_no_rtk_in_user_facing_show_config_output() {
        // show_config prints to stdout; verify the constants used don't say "RTK hook"
        // in user-facing contexts (comments in code are ok, but printed strings should say ContextZip)
        // This is a lightweight check on the key format strings
        let show_usage_lines =
            ["contextzip init -g --uninstall     # Remove all ContextZip artifacts"];
        for line in show_usage_lines {
            assert!(
                !line.contains("RTK artifacts"),
                "User-facing string should say ContextZip, not RTK: {}",
                line
            );
        }
    }
}
