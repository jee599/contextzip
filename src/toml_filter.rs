/// TOML-based filter DSL for ContextZip.
///
/// Provides a declarative pipeline of 8 stages that can be configured
/// via TOML files. Lookup priority (first match wins):
///   1. `.contextzip/filters.toml`              — project-local, committable with the repo
///   2. `~/.config/contextzip/filters.toml`     — user-global, applies to all projects
///   3. Built-in TOML                     — `src/filters/*.toml`, concatenated by build.rs and embedded at compile time
///   4. Passthrough                       — no match, handled by caller
///
/// `contextzip init` generates a commented template for both levels (project or global).
///
/// Environment variables:
///   - `CONTEXTZIP_NO_TOML=1`  — bypass TOML engine entirely (legacy: `RTK_NO_TOML=1`)
///   - `CONTEXTZIP_TOML_DEBUG=1` — print which filter matched and line counts to stderr (legacy: `RTK_TOML_DEBUG=1`)
///
/// Pipeline stages (applied in order):
///   1. strip_ansi           — remove ANSI escape codes
///   2. replace              — regex substitutions, line-by-line, chainable
///   3. match_output         — short-circuit: if blob matches a pattern, return message immediately
///   4. strip/keep_lines     — filter lines by regex
///   5. truncate_lines_at    — truncate each line to N chars
///   6. head/tail_lines      — keep first/last N lines
///   7. max_lines            — absolute line cap
///   8. on_empty             — message if result is empty
use lazy_static::lazy_static;
use regex::{Regex, RegexSet};
use serde::Deserialize;
use std::collections::BTreeMap;

lazy_static! {
    /// Matches `$VAR` or `${VAR}` where VAR is uppercase ASCII + digits + `_`.
    /// Used by `expand_env_vars` to substitute environment variables in TOML
    /// pattern fields before regex compilation. Lowercase / mixed-case names
    /// are intentionally not matched — env vars in regex patterns are almost
    /// always uppercase, and excluding mixed case avoids accidental capture
    /// of regex backreferences like `$1`.
    static ref ENV_VAR_RE: Regex =
        Regex::new(r"\$\{([A-Z_][A-Z0-9_]*)\}|\$([A-Z_][A-Z0-9_]*)").unwrap();
}

/// Substitute `$VAR` / `${VAR}` references in `s` with the corresponding env
/// values. Undefined variables become the empty string (silent).
fn expand_env_vars(s: &str) -> String {
    expand_env_vars_with(s, |k| std::env::var(k).ok())
}

/// Test seam for `expand_env_vars` — accepts an arbitrary lookup so unit tests
/// can avoid mutating the process environment.
fn expand_env_vars_with(s: &str, lookup: impl Fn(&str) -> Option<String>) -> String {
    ENV_VAR_RE
        .replace_all(s, |caps: &regex::Captures| {
            let name = caps.get(1).or_else(|| caps.get(2)).map(|m| m.as_str());
            name.and_then(&lookup).unwrap_or_default()
        })
        .into_owned()
}

/// Returns true if `platform_filter` (from TOML) is compatible with the
/// current OS. None means "any platform".
fn platform_matches(platform_filter: Option<&str>) -> bool {
    let Some(p) = platform_filter else {
        return true;
    };
    let current = current_os();
    match p {
        "any" | "" => true,
        "unix" => current == "macos" || current == "linux",
        other => other == current,
    }
}

fn current_os() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "other"
    }
}

// Built-in filters: concatenated from src/filters/*.toml by build.rs at compile time.
const BUILTIN_TOML: &str = include_str!(concat!(env!("OUT_DIR"), "/builtin_filters.toml"));

// ---------------------------------------------------------------------------
// Deserialization types (TOML schema)
// ---------------------------------------------------------------------------

/// A match-output rule: if `pattern` matches anywhere in the full output blob,
/// the filter short-circuits and returns `message` immediately.
/// First matching rule wins; remaining rules are not evaluated.
/// Optional `unless`: if this regex also matches the blob, the rule is skipped
/// (prevents short-circuiting when errors or warnings are present).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MatchOutputRule {
    pattern: String,
    message: String,
    #[serde(default)]
    unless: Option<String>,
}

/// A regex substitution applied line-by-line. Rules are chained sequentially:
/// rule N+1 operates on the output of rule N.
/// Backreferences (`$1`, `$2`, ...) are supported via the `regex` crate.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplaceRule {
    pattern: String,
    replacement: String,
}

/// An inline test case attached to a filter in the TOML.
/// Lives in `[[tests.<filter-name>]]` sections, separate from `[filters.*]`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TomlFilterTestDef {
    pub name: String,
    pub input: String,
    pub expected: String,
}

#[derive(Deserialize)]
struct TomlFilterFile {
    schema_version: u32,
    #[serde(default)]
    filters: BTreeMap<String, TomlFilterDef>,
    /// Inline tests keyed by filter name. Kept separate from `filters` so that
    /// `TomlFilterDef` can keep `deny_unknown_fields` without touching test data.
    #[serde(default)]
    tests: BTreeMap<String, Vec<TomlFilterTestDef>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TomlFilterDef {
    description: Option<String>,
    match_command: String,
    #[serde(default)]
    strip_ansi: bool,
    /// Regex substitutions, applied line-by-line before match_output (stage 2).
    #[serde(default)]
    replace: Vec<ReplaceRule>,
    /// Short-circuit rules: if the full output blob matches, return the message (stage 3).
    #[serde(default)]
    match_output: Vec<MatchOutputRule>,
    #[serde(default)]
    strip_lines_matching: Vec<String>,
    #[serde(default)]
    keep_lines_matching: Vec<String>,
    truncate_lines_at: Option<usize>,
    head_lines: Option<usize>,
    tail_lines: Option<usize>,
    max_lines: Option<usize>,
    on_empty: Option<String>,
    /// Restrict this filter to a specific OS. Accepts: `macos`, `linux`,
    /// `windows`, `unix` (= macos | linux), `any` / unset = all platforms.
    /// Filters that don't match the current OS are silently skipped at load
    /// time. Unknown values are treated as never-match.
    #[serde(default)]
    platform: Option<String>,
}

// ---------------------------------------------------------------------------
// Compiled types (post-validation, ready to use)
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct CompiledMatchOutputRule {
    pattern: Regex,
    message: String,
    /// If set and matches the blob, this rule is skipped (prevents swallowing errors).
    unless: Option<Regex>,
}

#[derive(Debug)]
struct CompiledReplaceRule {
    pattern: Regex,
    replacement: String,
}

#[derive(Debug)]
enum LineFilter {
    None,
    Strip(RegexSet),
    Keep(RegexSet),
}

/// A filter that has been parsed and compiled — all regexes are ready.
#[derive(Debug)]
pub struct CompiledFilter {
    pub name: String,
    #[allow(dead_code)]
    pub description: Option<String>,
    match_regex: Regex,
    strip_ansi: bool,
    replace: Vec<CompiledReplaceRule>,
    match_output: Vec<CompiledMatchOutputRule>,
    line_filter: LineFilter,
    truncate_lines_at: Option<usize>,
    head_lines: Option<usize>,
    tail_lines: Option<usize>,
    pub max_lines: Option<usize>,
    on_empty: Option<String>,
}

// ---------------------------------------------------------------------------
// Results for `contextzip verify`
// ---------------------------------------------------------------------------

/// Outcome of running a single inline test.
pub struct TestOutcome {
    pub filter_name: String,
    pub test_name: String,
    pub passed: bool,
    pub actual: String,
    pub expected: String,
}

/// Aggregated results from `run_filter_tests`.
pub struct VerifyResults {
    /// Individual test outcomes (all filters, or just the requested one).
    pub outcomes: Vec<TestOutcome>,
    /// Filter names that have no inline tests (used by `--require-all`).
    pub filters_without_tests: Vec<String>,
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

pub struct TomlFilterRegistry {
    pub filters: Vec<CompiledFilter>,
}

impl TomlFilterRegistry {
    /// Load registry from disk + built-in. Emits warnings to stderr on parse
    /// errors but never panics — bad files are silently ignored.
    fn load() -> Self {
        let mut filters = Vec::new();

        // Priority 1: project-local .contextzip/filters.toml (trust-gated)
        let project_filter_path = std::path::Path::new(".contextzip/filters.toml");
        if project_filter_path.exists() {
            let trust_status = crate::trust::check_trust(project_filter_path)
                .unwrap_or(crate::trust::TrustStatus::Untrusted);

            match trust_status {
                crate::trust::TrustStatus::Trusted | crate::trust::TrustStatus::EnvOverride => {
                    if let Ok(content) = std::fs::read_to_string(project_filter_path) {
                        match Self::parse_and_compile(&content, "project") {
                            Ok(f) => filters.extend(f),
                            Err(e) => {
                                eprintln!("[contextzip] warning: .contextzip/filters.toml: {}", e)
                            }
                        }
                    }
                }
                crate::trust::TrustStatus::Untrusted => {
                    eprintln!(
                        "[contextzip] WARNING: untrusted project filters (.contextzip/filters.toml)"
                    );
                    eprintln!("[contextzip] Filters NOT applied. Run `contextzip trust` to review and enable.");
                }
                crate::trust::TrustStatus::ContentChanged { .. } => {
                    eprintln!(
                        "[contextzip] WARNING: .contextzip/filters.toml changed since trusted."
                    );
                    eprintln!(
                        "[contextzip] Filters NOT applied. Run `contextzip trust` to re-review."
                    );
                }
            }
        }

        // Priority 2: user-global ~/.config/contextzip/filters.toml
        if let Some(config_dir) = dirs::config_dir() {
            let global_path = config_dir.join("contextzip").join("filters.toml");
            if let Ok(content) = std::fs::read_to_string(&global_path) {
                match Self::parse_and_compile(&content, "user-global") {
                    Ok(f) => filters.extend(f),
                    Err(e) => eprintln!("[contextzip] warning: {}: {}", global_path.display(), e),
                }
            }
        }

        // Priority 3: built-in (embedded at compile time)
        let builtin = BUILTIN_TOML;
        match Self::parse_and_compile(builtin, "builtin") {
            Ok(f) => filters.extend(f),
            Err(e) => eprintln!("[contextzip] warning: builtin filters: {}", e),
        }

        TomlFilterRegistry { filters }
    }

    fn parse_and_compile(content: &str, source: &str) -> Result<Vec<CompiledFilter>, String> {
        let file: TomlFilterFile = toml::from_str(content)
            .map_err(|e| format!("TOML parse error in {}: {}", source, e))?;

        if file.schema_version != 1 {
            return Err(format!(
                "unsupported schema_version {} in {} (expected 1)",
                file.schema_version, source
            ));
        }

        let mut compiled = Vec::new();
        for (name, def) in file.filters {
            // Skip filters tagged for a different OS — silently. Filter authors
            // can write `platform = "linux"` next to a Linux-specific filter
            // and ContextZip just ignores it on macOS / Windows.
            if !platform_matches(def.platform.as_deref()) {
                continue;
            }
            match compile_filter(name.clone(), def) {
                Ok(f) => compiled.push(f),
                Err(e) => eprintln!(
                    "[contextzip] warning: filter '{}' in {}: {}",
                    name, source, e
                ),
            }
        }
        Ok(compiled)
    }
}

/// Commands already handled by dedicated Rust modules (routed by Clap before TOML).
/// A TOML filter whose match_command matches one of these will never activate —
/// Clap routes the command before `run_fallback()` is reached.
const RUST_HANDLED_COMMANDS: &[&str] = &[
    "ls",
    "tree",
    "read",
    "smart",
    "git",
    "gh",
    "aws",
    "psql",
    "pnpm",
    "err",
    "test",
    "json",
    "deps",
    "env",
    "find",
    "diff",
    "log",
    "docker",
    "kubectl",
    "summary",
    "grep",
    "init",
    "wget",
    "wc",
    "gain",
    "config",
    "vitest",
    "prisma",
    "tsc",
    "next",
    "lint",
    "prettier",
    "format",
    "playwright",
    "cargo",
    "npm",
    "npx",
    "curl",
    "discover",
    "ruff",
    "pytest",
    "mypy",
    "pip",
    "go",
    "golangci-lint",
    "rewrite",
    "proxy",
    "verify",
    "learn",
];

fn compile_filter(name: String, def: TomlFilterDef) -> Result<CompiledFilter, String> {
    // Mutual exclusion: strip and keep cannot both be set
    if !def.strip_lines_matching.is_empty() && !def.keep_lines_matching.is_empty() {
        return Err("strip_lines_matching and keep_lines_matching are mutually exclusive".into());
    }

    // Pre-expand env-var references in every pattern field so filter authors can
    // write `$HOME`, `$CI_COMMIT_SHA`, etc. once instead of hardcoding paths or
    // splitting filters per environment.
    let expanded_match_command = expand_env_vars(&def.match_command);
    let match_regex = Regex::new(&expanded_match_command)
        .map_err(|e| format!("invalid match_command regex: {}", e))?;

    // Shadow warning: if match_command matches a Rust-handled command, this filter
    // will never activate (Clap routes before run_fallback). Warn the author.
    for cmd in RUST_HANDLED_COMMANDS {
        if match_regex.is_match(cmd) {
            eprintln!(
                "[contextzip] warning: filter '{}' match_command matches '{}' which is already \
                 handled by a Rust module — this filter will never activate for that command",
                name, cmd
            );
            break;
        }
    }

    let replace = def
        .replace
        .into_iter()
        .map(|r| {
            let expanded = expand_env_vars(&r.pattern);
            Regex::new(&expanded)
                .map(|pattern| CompiledReplaceRule {
                    pattern,
                    replacement: expand_env_vars(&r.replacement),
                })
                .map_err(|e| format!("invalid replace pattern '{}': {}", r.pattern, e))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let match_output = def
        .match_output
        .into_iter()
        .map(|r| -> Result<CompiledMatchOutputRule, String> {
            let expanded = expand_env_vars(&r.pattern);
            let pattern = Regex::new(&expanded)
                .map_err(|e| format!("invalid match_output pattern '{}': {}", r.pattern, e))?;
            let unless = r
                .unless
                .as_deref()
                .map(|u| {
                    let expanded_u = expand_env_vars(u);
                    Regex::new(&expanded_u)
                        .map_err(|e| format!("invalid match_output unless pattern '{}': {}", u, e))
                })
                .transpose()?;
            Ok(CompiledMatchOutputRule {
                pattern,
                message: r.message,
                unless,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let strip_expanded: Vec<String> = def
        .strip_lines_matching
        .iter()
        .map(|s| expand_env_vars(s))
        .collect();
    let keep_expanded: Vec<String> = def
        .keep_lines_matching
        .iter()
        .map(|s| expand_env_vars(s))
        .collect();

    let line_filter = if !strip_expanded.is_empty() {
        let set = RegexSet::new(&strip_expanded)
            .map_err(|e| format!("invalid strip_lines_matching regex: {}", e))?;
        LineFilter::Strip(set)
    } else if !keep_expanded.is_empty() {
        let set = RegexSet::new(&keep_expanded)
            .map_err(|e| format!("invalid keep_lines_matching regex: {}", e))?;
        LineFilter::Keep(set)
    } else {
        LineFilter::None
    };

    Ok(CompiledFilter {
        name,
        description: def.description,
        match_regex,
        strip_ansi: def.strip_ansi,
        replace,
        match_output,
        line_filter,
        truncate_lines_at: def.truncate_lines_at,
        head_lines: def.head_lines,
        tail_lines: def.tail_lines,
        max_lines: def.max_lines,
        on_empty: def.on_empty,
    })
}

// ---------------------------------------------------------------------------
// Singleton (lazy-loaded, one-time cost)
// ---------------------------------------------------------------------------

lazy_static! {
    static ref REGISTRY: TomlFilterRegistry = TomlFilterRegistry::load();
}

// ---------------------------------------------------------------------------
// Public API — pure functions (testable without global state)
// ---------------------------------------------------------------------------

/// Find the first matching filter in a slice. O(N) on the number of filters.
/// Tests should call this directly with a local filter list.
pub fn find_filter_in<'a>(
    command: &str,
    filters: &'a [CompiledFilter],
) -> Option<&'a CompiledFilter> {
    filters.iter().find(|f| f.match_regex.is_match(command))
}

/// Apply a compiled filter pipeline to raw stdout. Pure String -> String.
///
/// Pipeline stages (in order):
///   1. strip_ansi           — remove ANSI escape codes
///   2. replace              — regex substitutions, line-by-line, chainable
///   3. match_output         — short-circuit if blob matches a pattern
///   4. strip/keep_lines     — filter lines by regex
///   5. truncate_lines_at    — truncate each line to N chars
///   6. head/tail_lines      — keep first/last N lines
///   7. max_lines            — absolute line cap
///   8. on_empty             — message if result is empty
pub fn apply_filter(filter: &CompiledFilter, stdout: &str) -> String {
    let mut lines: Vec<String> = stdout.lines().map(String::from).collect();

    // 1. strip_ansi
    if filter.strip_ansi {
        lines = lines
            .into_iter()
            .map(|l| crate::utils::strip_ansi(&l))
            .collect();
    }

    // 2. replace — line-by-line, rules chained sequentially
    if !filter.replace.is_empty() {
        lines = lines
            .into_iter()
            .map(|mut line| {
                for rule in &filter.replace {
                    line = rule
                        .pattern
                        .replace_all(&line, rule.replacement.as_str())
                        .into_owned();
                }
                line
            })
            .collect();
    }

    // 3. match_output — short-circuit on full blob match (first rule wins)
    //    If `unless` is set and also matches the blob, the rule is skipped.
    if !filter.match_output.is_empty() {
        let blob = lines.join("\n");
        for rule in &filter.match_output {
            if rule.pattern.is_match(&blob) {
                if let Some(ref unless_re) = rule.unless {
                    if unless_re.is_match(&blob) {
                        continue; // errors/warnings present — skip this rule
                    }
                }
                return rule.message.clone();
            }
        }
    }

    // 4. strip OR keep (mutually exclusive)
    match &filter.line_filter {
        LineFilter::Strip(set) => lines.retain(|l| !set.is_match(l)),
        LineFilter::Keep(set) => lines.retain(|l| set.is_match(l)),
        LineFilter::None => {}
    }

    // 5. truncate_lines_at — uses utils::truncate (unicode-safe)
    if let Some(max_chars) = filter.truncate_lines_at {
        lines = lines
            .into_iter()
            .map(|l| crate::utils::truncate(&l, max_chars))
            .collect();
    }

    // 6. head + tail
    let total = lines.len();
    if let (Some(head), Some(tail)) = (filter.head_lines, filter.tail_lines) {
        if total > head + tail {
            let mut result = lines[..head].to_vec();
            result.push(format!("... ({} lines omitted)", total - head - tail));
            result.extend_from_slice(&lines[total - tail..]);
            lines = result;
        }
    } else if let Some(head) = filter.head_lines {
        if total > head {
            lines.truncate(head);
            lines.push(format!("... ({} lines omitted)", total - head));
        }
    } else if let Some(tail) = filter.tail_lines {
        if total > tail {
            let omitted = total - tail;
            lines = lines[omitted..].to_vec();
            lines.insert(0, format!("... ({} lines omitted)", omitted));
        }
    }

    // 7. max_lines — absolute cap applied after head/tail (includes omit messages)
    if let Some(max) = filter.max_lines {
        if lines.len() > max {
            let truncated = lines.len() - max;
            lines.truncate(max);
            lines.push(format!("... ({} lines truncated)", truncated));
        }
    }

    // 8. on_empty
    let result = lines.join("\n");
    if result.trim().is_empty() {
        if let Some(ref msg) = filter.on_empty {
            return msg.clone();
        }
    }

    result
}

// ---------------------------------------------------------------------------
// contextzip verify — inline test execution
// ---------------------------------------------------------------------------

/// Run inline tests from loaded TOML files (builtin + project-local).
///
/// - `filter_name_opt`: if `Some`, only run tests for that filter name.
/// - Returns `VerifyResults` with all outcomes and filters that have no tests.
pub fn run_filter_tests(filter_name_opt: Option<&str>) -> VerifyResults {
    let mut outcomes = Vec::new();
    let mut all_filter_names: Vec<String> = Vec::new();
    let mut tested_filter_names: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    let builtin = BUILTIN_TOML;
    collect_test_outcomes(
        builtin,
        filter_name_opt,
        &mut outcomes,
        &mut all_filter_names,
        &mut tested_filter_names,
    );

    // Trust-gated: only verify project-local filters if trusted (SA-2025-RTK-002)
    let project_path = std::path::Path::new(".contextzip/filters.toml");
    if project_path.exists() {
        let trust_status =
            crate::trust::check_trust(project_path).unwrap_or(crate::trust::TrustStatus::Untrusted);
        match trust_status {
            crate::trust::TrustStatus::Trusted | crate::trust::TrustStatus::EnvOverride => {
                if let Ok(content) = std::fs::read_to_string(project_path) {
                    collect_test_outcomes(
                        &content,
                        filter_name_opt,
                        &mut outcomes,
                        &mut all_filter_names,
                        &mut tested_filter_names,
                    );
                }
            }
            _ => {
                eprintln!("[contextzip] WARNING: untrusted project filters skipped in verify");
            }
        }
    }

    let filters_without_tests = all_filter_names
        .into_iter()
        .filter(|name| {
            // When a specific filter is requested, only report that one as missing tests
            filter_name_opt.is_none_or(|f| name == f)
        })
        .filter(|name| !tested_filter_names.contains(name))
        .collect();

    VerifyResults {
        outcomes,
        filters_without_tests,
    }
}

fn collect_test_outcomes(
    content: &str,
    filter_name_opt: Option<&str>,
    outcomes: &mut Vec<TestOutcome>,
    all_filter_names: &mut Vec<String>,
    tested_filter_names: &mut std::collections::HashSet<String>,
) {
    let file: TomlFilterFile = match toml::from_str(content) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "[contextzip] warning: TOML parse error during verify: {}",
                e
            );
            return;
        }
    };

    // Compile all filters and track their names
    let mut compiled_filters: BTreeMap<String, CompiledFilter> = BTreeMap::new();
    for (name, def) in file.filters {
        all_filter_names.push(name.clone());
        match compile_filter(name.clone(), def) {
            Ok(f) => {
                compiled_filters.insert(name, f);
            }
            Err(e) => eprintln!(
                "[contextzip] warning: filter '{}' compilation error: {}",
                name, e
            ),
        }
    }

    // Run tests
    for (filter_name, tests) in file.tests {
        if let Some(name) = filter_name_opt {
            if filter_name != name {
                continue;
            }
        }

        tested_filter_names.insert(filter_name.clone());

        let compiled = match compiled_filters.get(&filter_name) {
            Some(f) => f,
            None => {
                eprintln!(
                    "[contextzip] warning: [[tests.{}]] references unknown filter",
                    filter_name
                );
                continue;
            }
        };

        for test in tests {
            let actual = apply_filter(compiled, &test.input);
            // Trim trailing newlines: TOML multiline strings end with a newline
            let actual_cmp = actual.trim_end_matches('\n').to_string();
            let expected_cmp = test.expected.trim_end_matches('\n').to_string();
            outcomes.push(TestOutcome {
                filter_name: filter_name.clone(),
                test_name: test.name,
                passed: actual_cmp == expected_cmp,
                actual: actual_cmp,
                expected: expected_cmp,
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Convenience wrapper (uses singleton — for run_fallback)
// ---------------------------------------------------------------------------

/// Find a matching filter from the global registry. Initialises the registry
/// lazily on first call. Returns `None` if no filter matches.
pub fn find_matching_filter(command: &str) -> Option<&'static CompiledFilter> {
    if std::env::var("CONTEXTZIP_TOML_DEBUG")
        .or_else(|_| std::env::var("RTK_TOML_DEBUG"))
        .is_ok()
    {
        eprintln!(
            "[contextzip:toml] looking up filter for: {:?} ({} filters loaded)",
            command,
            REGISTRY.filters.len()
        );
    }
    let result = find_filter_in(command, &REGISTRY.filters);
    if std::env::var("CONTEXTZIP_TOML_DEBUG")
        .or_else(|_| std::env::var("RTK_TOML_DEBUG"))
        .is_ok()
    {
        match result {
            Some(f) => eprintln!("[contextzip:toml] matched filter: '{}'", f.name),
            None => eprintln!("[contextzip:toml] no filter matched — passthrough"),
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: build a CompiledFilter from inline TOML for tests.
    // Never touches the lazy_static registry.
    fn make_filters(toml: &str) -> Vec<CompiledFilter> {
        TomlFilterRegistry::parse_and_compile(toml, "test").expect("test TOML should be valid")
    }

    fn first_filter(toml: &str) -> CompiledFilter {
        make_filters(toml)
            .into_iter()
            .next()
            .expect("expected at least one filter")
    }

    // --- Pipeline primitives (existing) ---

    #[test]
    fn test_strip_ansi_removes_codes() {
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
strip_ansi = true
"#,
        );
        let out = apply_filter(&f, "\x1b[31mError\x1b[0m\nnormal");
        assert_eq!(out, "Error\nnormal");
    }

    #[test]
    fn test_strip_lines_matching_basic() {
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
strip_lines_matching = ["^noise", "^verbose"]
"#,
        );
        let input = "noise line\nkeep this\nverbose stuff\nalso keep";
        let out = apply_filter(&f, input);
        assert_eq!(out, "keep this\nalso keep");
    }

    #[test]
    fn test_keep_lines_matching_basic() {
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
keep_lines_matching = ["^PASS", "^FAIL"]
"#,
        );
        let input = "PASS test_a\nsome noise\nFAIL test_b\nmore noise";
        let out = apply_filter(&f, input);
        assert_eq!(out, "PASS test_a\nFAIL test_b");
    }

    #[test]
    fn test_truncate_lines_at_unicode_safe() {
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
truncate_lines_at = 5
"#,
        );
        // utils::truncate(s, 5) takes 2 chars + "..." when len > 5
        // "hello" = 5 chars exactly, stays unchanged
        // "日本語xyz" = 6 chars, truncated to "日本..." (take 2 + "...")
        let out = apply_filter(&f, "hello\n日本語xyz");
        assert_eq!(out, "hello\n日本...");
    }

    #[test]
    fn test_head_lines() {
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
head_lines = 2
"#,
        );
        let input = "a\nb\nc\nd\ne";
        let out = apply_filter(&f, input);
        assert!(out.starts_with("a\nb\n"));
        assert!(out.contains("3 lines omitted"));
    }

    #[test]
    fn test_tail_lines() {
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
tail_lines = 2
"#,
        );
        let input = "a\nb\nc\nd\ne";
        let out = apply_filter(&f, input);
        assert!(out.contains("3 lines omitted"));
        assert!(out.ends_with("d\ne"));
    }

    #[test]
    fn test_head_and_tail_combined() {
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
head_lines = 2
tail_lines = 2
"#,
        );
        let input = "a\nb\nc\nd\ne\nf";
        let out = apply_filter(&f, input);
        assert!(out.starts_with("a\nb\n"));
        assert!(out.contains("2 lines omitted"));
        assert!(out.ends_with("e\nf"));
    }

    #[test]
    fn test_max_lines_counts_omit_message() {
        // max_lines applied AFTER head — the "omitted" message counts as a line
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
max_lines = 3
"#,
        );
        let input = "a\nb\nc\nd\ne";
        let out = apply_filter(&f, input);
        let line_count = out.lines().count();
        // 3 content lines + 1 truncated message = 4 lines output
        assert_eq!(line_count, 4);
        assert!(out.contains("lines truncated"));
    }

    #[test]
    fn test_on_empty_when_all_filtered() {
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
strip_lines_matching = [".*"]
on_empty = "nothing left"
"#,
        );
        let out = apply_filter(&f, "line1\nline2");
        assert_eq!(out, "nothing left");
    }

    #[test]
    fn test_on_empty_not_triggered_when_output_remains() {
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
keep_lines_matching = ["keep"]
on_empty = "nothing left"
"#,
        );
        let out = apply_filter(&f, "keep this\nnoise");
        assert_eq!(out, "keep this");
    }

    #[test]
    fn test_full_pipeline_order() {
        // Verify all 8 stages fire in order on a single input
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
strip_ansi = true
strip_lines_matching = ["^noise"]
truncate_lines_at = 10
head_lines = 3
max_lines = 4
on_empty = "empty"
"#,
        );
        let input =
            "\x1b[31mred line\x1b[0m\nnoise skip\nkeep one\nkeep two\nkeep three\nkeep four";
        let out = apply_filter(&f, input);
        // After strip_ansi: "red line", strip noise: removed, head 3 from remaining 4 lines
        assert!(out.contains("red line"));
        assert!(!out.contains("noise skip"));
        assert!(out.contains("lines omitted") || out.contains("lines truncated"));
    }

    // --- Validation ---

    #[test]
    fn test_mutual_exclusion_strip_keep_errors() {
        let result = make_filters(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
strip_lines_matching = ["a"]
keep_lines_matching = ["b"]
"#,
        );
        // The filter should be skipped (warning emitted), resulting in empty list
        assert!(result.is_empty());
    }

    #[test]
    fn test_invalid_regex_returns_err() {
        let result = make_filters(
            r#"
schema_version = 1
[filters.f]
match_command = "["
"#,
        );
        assert!(result.is_empty());
    }

    #[test]
    fn test_schema_version_mismatch_errors() {
        let result = TomlFilterRegistry::parse_and_compile(
            r#"schema_version = 99
[filters.f]
match_command = "^cmd"
"#,
            "test",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_field_typo_errors() {
        // deny_unknown_fields should catch this
        let result = TomlFilterRegistry::parse_and_compile(
            r#"schema_version = 1
[filters.f]
match_command = "^cmd"
strip_ansi_typo = true
"#,
            "test",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_filter_passthrough() {
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
"#,
        );
        let input = "line1\nline2\nline3";
        let out = apply_filter(&f, input);
        assert_eq!(out, input);
    }

    // --- Registry / find ---

    #[test]
    fn test_builtin_filters_compile() {
        // Compile-time safety: panics if any src/filters/*.toml is broken
        let builtin = BUILTIN_TOML;
        let result = TomlFilterRegistry::parse_and_compile(builtin, "builtin");
        assert!(
            result.is_ok(),
            "builtin filters failed to compile: {:?}",
            result
        );
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_find_filter_matches_terraform() {
        let filters = make_filters(
            r#"
schema_version = 1
[filters.terraform-plan]
match_command = "^terraform\\s+plan"
strip_ansi = true
"#,
        );
        let found = find_filter_in("terraform plan -out=tfplan", &filters);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "terraform-plan");
    }

    #[test]
    fn test_find_filter_no_match_returns_none() {
        let filters = make_filters(
            r#"
schema_version = 1
[filters.f]
match_command = "^terraform"
"#,
        );
        let found = find_filter_in("kubectl get pods", &filters);
        assert!(found.is_none());
    }

    #[test]
    fn test_project_filters_priority_over_builtin() {
        // Project filter has same name but different max_lines — project wins
        let project = make_filters(
            r#"
schema_version = 1
[filters.make]
match_command = "^make\\b"
max_lines = 999
"#,
        );
        let builtin = make_filters(BUILTIN_TOML);

        // Simulate the registry: project first
        let mut all = project;
        all.extend(builtin);

        let found = find_filter_in("make all", &all).expect("should match");
        assert_eq!(found.name, "make");
        // The first (project) match has max_lines=999
        assert_eq!(found.max_lines, Some(999));
    }

    // --- Token savings ---

    #[test]
    fn test_terraform_savings_above_60pct() {
        let filters = make_filters(BUILTIN_TOML);
        let filter = find_filter_in("terraform plan", &filters).expect("terraform-plan built-in");

        // Inline fixture: realistic terraform plan with many Refreshing state lines (noise).
        // Real infra refreshes 30+ resources; the plan section is small.
        // All Refreshing/lock/blank/unchanged lines are stripped -> >60% savings.
        let input = concat!(
            "Acquiring state lock. This may take a few moments...\n",
            "Refreshing state... [id=vpc-0a1b2c3d]\n",
            "Refreshing state... [id=subnet-11111111]\n",
            "Refreshing state... [id=subnet-22222222]\n",
            "Refreshing state... [id=subnet-33333333]\n",
            "Refreshing state... [id=subnet-44444444]\n",
            "Refreshing state... [id=igw-aabbccdd]\n",
            "Refreshing state... [id=rtb-aabbccdd]\n",
            "Refreshing state... [id=rtb-11223344]\n",
            "Refreshing state... [id=sg-00112233]\n",
            "Refreshing state... [id=sg-44556677]\n",
            "Refreshing state... [id=sg-88990011]\n",
            "Refreshing state... [id=nacl-00aabbcc]\n",
            "Refreshing state... [id=acm-arn:us-east-1:cert/abc]\n",
            "Refreshing state... [id=Z1234567890ABC]\n",
            "Refreshing state... [id=alb-arn:my-alb]\n",
            "Refreshing state... [id=tg-arn:my-tg]\n",
            "Refreshing state... [id=db-ABCDEFGHIJKLMNO]\n",
            "Refreshing state... [id=rds-cluster:my-aurora]\n",
            "Refreshing state... [id=elasticache:my-cluster]\n",
            "Refreshing state... [id=lambda:my-api-function]\n",
            "Refreshing state... [id=lambda:my-worker]\n",
            "Refreshing state... [id=iam-role:my-lambda-role]\n",
            "Refreshing state... [id=iam-role:my-ecs-role]\n",
            "Refreshing state... [id=s3:::my-app-assets]\n",
            "Refreshing state... [id=s3:::my-app-logs]\n",
            "Refreshing state... [id=cloudfront:ABCDEFGHIJK]\n",
            "Refreshing state... [id=ssm:/my/app/db-url]\n",
            "Refreshing state... [id=ssm:/my/app/api-key]\n",
            "Refreshing state... [id=secretsmanager:my-secret]\n",
            "Releasing state lock. This may take a few moments...\n",
            "\n",
            "Terraform will perform the following actions:\n",
            "\n",
            "  # aws_instance.web will be created\n",
            "  + resource \"aws_instance\" \"web\" {\n",
            "      + ami           = \"ami-0c55b159cbfafe1f0\"\n",
            "      + instance_type = \"t3.micro\"\n",
            "    }\n",
            "\n",
            "Plan: 1 to add, 0 to change, 0 to destroy.\n",
        );
        let out = apply_filter(filter, input);
        let input_words = input.split_whitespace().count();
        let out_words = out.split_whitespace().count();
        let savings = 100.0 - (out_words as f64 / input_words as f64 * 100.0);
        assert!(
            savings >= 60.0,
            "terraform-plan filter: expected >=60% savings, got {:.1}% (in={} out={})",
            savings,
            input_words,
            out_words
        );
    }

    #[test]
    fn test_make_savings_above_60pct() {
        let filters = make_filters(BUILTIN_TOML);
        let filter = find_filter_in("make all", &filters).expect("make built-in");

        let input = r#"make[1]: Entering directory '/home/user/project'
make[2]: Entering directory '/home/user/project/src'
gcc -O2 -Wall -c foo.c -o foo.o

make[2]: Nothing to be done for 'install'.
make[3]: Entering directory '/home/user/project/src/lib'
ar rcs libfoo.a foo.o bar.o baz.o
make[3]: Leaving directory '/home/user/project/src/lib'
make[2]: Leaving directory '/home/user/project/src'

make[1]: Leaving directory '/home/user/project'
gcc -O2 -Wall -c bar.c -o bar.o

gcc -O2 -Wall -c baz.c -o baz.o

make[1]: Entering directory '/home/user/project/test'
make[2]: Entering directory '/home/user/project/test/unit'
./run_tests --verbose
make[2]: Nothing to be done for 'check'.
make[2]: Leaving directory '/home/user/project/test/unit'
make[1]: Leaving directory '/home/user/project/test'

ld -o myapp foo.o bar.o baz.o -lfoo

make[1]: Entering directory '/home/user/project/docs'
doxygen Doxyfile
make[1]: Leaving directory '/home/user/project/docs'
"#;
        let out = apply_filter(filter, input);
        let input_words = input.split_whitespace().count();
        let out_words = out.split_whitespace().count();
        let savings = 100.0 - (out_words as f64 / input_words as f64 * 100.0);
        assert!(
            savings >= 60.0,
            "make filter: expected >=60% savings, got {:.1}% (in={} out={})",
            savings,
            input_words,
            out_words
        );
    }

    // --- Edge cases ---

    #[test]
    fn test_empty_input() {
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
strip_lines_matching = [".*"]
"#,
        );
        let out = apply_filter(&f, "");
        assert_eq!(out, "");
    }

    #[test]
    fn test_unicode_preserved() {
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
strip_lines_matching = ["^noise"]
"#,
        );
        let out = apply_filter(&f, "日本語テスト\nnoise\n中文内容");
        assert_eq!(out, "日本語テスト\n中文内容");
    }

    // --- match_output tests (PR1) ---

    #[test]
    fn test_match_output_basic_short_circuit() {
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
match_output = [
  { pattern = "Switched to branch", message = "ok" },
]
"#,
        );
        let out = apply_filter(&f, "Switched to branch 'main'");
        assert_eq!(out, "ok");
    }

    #[test]
    fn test_match_output_second_rule_matches() {
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
match_output = [
  { pattern = "Switched to branch", message = "switched" },
  { pattern = "Already on", message = "already" },
]
"#,
        );
        let out = apply_filter(&f, "Already on 'main'");
        assert_eq!(out, "already");
    }

    #[test]
    fn test_match_output_no_match_pipeline_continues() {
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
match_output = [
  { pattern = "Switched to branch", message = "ok" },
]
strip_lines_matching = ["^noise"]
"#,
        );
        let out = apply_filter(&f, "noise\nkeep this");
        // No match_output match, pipeline continues and strips noise
        assert_eq!(out, "keep this");
    }

    #[test]
    fn test_match_output_strip_ansi_before_match() {
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
strip_ansi = true
match_output = [
  { pattern = "Switched to branch", message = "ok" },
]
"#,
        );
        // ANSI stripped before match_output check (stage 1 before stage 3)
        let out = apply_filter(&f, "\x1b[32mSwitched to branch\x1b[0m 'main'");
        assert_eq!(out, "ok");
    }

    #[test]
    fn test_match_output_no_match_then_on_empty() {
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
match_output = [
  { pattern = "Switched", message = "ok" },
]
strip_lines_matching = [".*"]
on_empty = "nothing"
"#,
        );
        // No match_output match; pipeline strips all lines; on_empty fires
        let out = apply_filter(&f, "foo bar baz");
        assert_eq!(out, "nothing");
    }

    #[test]
    fn test_match_output_invalid_regex_rejected() {
        let result = make_filters(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
match_output = [
  { pattern = "[invalid", message = "ok" },
]
"#,
        );
        assert!(result.is_empty());
    }

    // --- match_output unless tests (PR3) ---

    #[test]
    fn test_match_output_unless_blocks_short_circuit_when_errors_present() {
        // "total size is" matches, but "error" also matches — unless fires, rule is skipped.
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^rsync"
match_output = [
  { pattern = "total size is", message = "ok (synced)", unless = "error|failed" },
]
"#,
        );
        let input = "rsync: [sender] error\ntotal size is 1000  speedup is 3.33\n";
        let out = apply_filter(&f, input);
        // Should NOT return "ok (synced)" because "error" matches the unless pattern
        assert_ne!(
            out.trim(),
            "ok (synced)",
            "unless should have blocked short-circuit when errors are present"
        );
        // The raw lines should pass through (no further strip rules in this filter)
        assert!(out.contains("error"));
    }

    #[test]
    fn test_match_output_unless_allows_short_circuit_when_no_errors() {
        // "total size is" matches and "error" does NOT appear — unless does not fire, rule wins.
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^rsync"
match_output = [
  { pattern = "total size is", message = "ok (synced)", unless = "error|failed" },
]
"#,
        );
        let input = "file.txt\ntotal size is 98765  speedup is 77.31\n";
        let out = apply_filter(&f, input);
        assert_eq!(out.trim(), "ok (synced)");
    }

    #[test]
    fn test_match_output_unless_falls_through_to_next_rule() {
        // First rule blocked by unless; second rule (no unless) should match.
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
match_output = [
  { pattern = "success", message = "ok", unless = "error" },
  { pattern = "success", message = "ok with warnings" },
]
"#,
        );
        let input = "success\nerror: something went wrong\n";
        let out = apply_filter(&f, input);
        // First rule skipped (unless matched), second rule (no unless) fires
        assert_eq!(out.trim(), "ok with warnings");
    }

    #[test]
    fn test_match_output_unless_no_field_behaves_like_before() {
        // When unless is absent, behaviour is identical to original (no regression).
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
match_output = [
  { pattern = "Build complete", message = "ok (build complete)" },
]
"#,
        );
        let out = apply_filter(&f, "Build complete!\n");
        assert_eq!(out.trim(), "ok (build complete)");
    }

    #[test]
    fn test_match_output_unless_invalid_regex_rejected() {
        let result = make_filters(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
match_output = [
  { pattern = "success", message = "ok", unless = "[invalid" },
]
"#,
        );
        assert!(result.is_empty());
    }

    // --- replace tests (PR1) ---

    #[test]
    fn test_replace_basic_all_occurrences() {
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
replace = [
  { pattern = "foo", replacement = "bar" },
]
"#,
        );
        let out = apply_filter(&f, "foo baz foo\nfoo");
        assert_eq!(out, "bar baz bar\nbar");
    }

    #[test]
    fn test_replace_chaining_sequential() {
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
replace = [
  { pattern = "aaa", replacement = "bbb" },
  { pattern = "bbb", replacement = "ccc" },
]
"#,
        );
        // Rule 2 operates on the output of rule 1
        let out = apply_filter(&f, "aaa");
        assert_eq!(out, "ccc");
    }

    #[test]
    fn test_replace_backreferences() {
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
replace = [
  { pattern = "(\\w+):(\\w+)", replacement = "$2:$1" },
]
"#,
        );
        let out = apply_filter(&f, "hello:world");
        assert_eq!(out, "world:hello");
    }

    #[test]
    fn test_replace_then_strip_interaction() {
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
replace = [
  { pattern = "noise", replacement = "DROPPED" },
]
strip_lines_matching = ["^DROPPED"]
"#,
        );
        // replace transforms "noise line" -> "DROPPED line", strip removes it
        let out = apply_filter(&f, "noise line\nkeep this");
        assert_eq!(out, "keep this");
    }

    #[test]
    fn test_replace_empty_input_noop() {
        let f = first_filter(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
replace = [
  { pattern = "foo", replacement = "bar" },
]
"#,
        );
        let out = apply_filter(&f, "");
        assert_eq!(out, "");
    }

    #[test]
    fn test_replace_invalid_regex_rejected() {
        let result = make_filters(
            r#"
schema_version = 1
[filters.f]
match_command = "^cmd"
replace = [
  { pattern = "[invalid", replacement = "bar" },
]
"#,
        );
        assert!(result.is_empty());
    }

    // --- verify (PR2) ---

    #[test]
    fn test_run_filter_tests_passes_on_correct_expected() {
        let content = r#"
schema_version = 1

[filters.make]
match_command = "^make\\b"
strip_lines_matching = ["^make\\[\\d+\\]:"]

[[tests.make]]
name = "strips entering/leaving lines"
input = """
make[1]: Entering directory '/home/user'
gcc -O2 foo.c
make[1]: Leaving directory '/home/user'
"""
expected = """
gcc -O2 foo.c
"""
"#;
        let mut outcomes = Vec::new();
        let mut all_names = Vec::new();
        let mut tested = std::collections::HashSet::new();
        collect_test_outcomes(content, None, &mut outcomes, &mut all_names, &mut tested);
        assert_eq!(outcomes.len(), 1);
        assert!(
            outcomes[0].passed,
            "test should pass: {:?}",
            outcomes[0].actual
        );
    }

    #[test]
    fn test_run_filter_tests_fails_on_wrong_expected() {
        let content = r#"
schema_version = 1

[filters.make]
match_command = "^make\\b"
strip_lines_matching = ["^make\\[\\d+\\]:"]

[[tests.make]]
name = "wrong expected"
input = "make[1]: Entering\ngcc foo.c"
expected = "wrong output"
"#;
        let mut outcomes = Vec::new();
        let mut all_names = Vec::new();
        let mut tested = std::collections::HashSet::new();
        collect_test_outcomes(content, None, &mut outcomes, &mut all_names, &mut tested);
        assert_eq!(outcomes.len(), 1);
        assert!(!outcomes[0].passed);
    }

    #[test]
    fn test_filters_without_tests_detected() {
        let content = r#"
schema_version = 1

[filters.make]
match_command = "^make\\b"
"#;
        let mut outcomes = Vec::new();
        let mut all_names = Vec::new();
        let mut tested = std::collections::HashSet::new();
        collect_test_outcomes(content, None, &mut outcomes, &mut all_names, &mut tested);
        // No tests defined, but filter exists
        assert_eq!(outcomes.len(), 0);
        assert!(all_names.contains(&"make".to_string()));
        assert!(!tested.contains("make"));
    }

    // --- Multi-file architecture tests (build.rs) ---

    /// Verify BUILTIN_TOML was generated with the correct schema_version header.
    /// build.rs injects it — if the const is somehow stale this fails immediately.
    #[test]
    fn test_builtin_toml_has_schema_version() {
        assert!(
            BUILTIN_TOML.contains("schema_version = 1"),
            "BUILTIN_TOML must start with 'schema_version = 1' (injected by build.rs)"
        );
    }

    /// Verify every expected filter name is present in BUILTIN_TOML.
    /// This is the safeguard against accidentally deleting a filter file.
    #[test]
    fn test_builtin_all_expected_filters_present() {
        let filters = make_filters(BUILTIN_TOML);
        let names: std::collections::HashSet<&str> =
            filters.iter().map(|f| f.name.as_str()).collect();

        let expected = [
            "ansible-playbook",
            "brew-install",
            "composer-install",
            "df",
            "dotnet-build",
            "du",
            "fail2ban-client",
            "gcloud",
            "hadolint",
            "helm",
            "iptables",
            "make",
            "markdownlint",
            "mix-compile",
            "mix-format",
            "mvn-build",
            "ping",
            "pio-run",
            "poetry-install",
            "pre-commit",
            "ps",
            "quarto-render",
            "rsync",
            "shellcheck",
            "shopify-theme",
            "sops",
            "swift-build",
            "systemctl-status",
            "terraform-plan",
            "tofu-fmt",
            "tofu-init",
            "tofu-plan",
            "tofu-validate",
            "trunk-build",
            "uv-sync",
            "yamllint",
        ];

        for name in &expected {
            assert!(
                names.contains(name),
                "Built-in filter '{}' is missing — was its .toml file deleted from src/filters/?",
                name
            );
        }
    }

    /// Verify the exact count of built-in filters.
    /// Fails if a file is added/removed without updating this test.
    #[test]
    fn test_builtin_filter_count() {
        let filters = make_filters(BUILTIN_TOML);
        assert_eq!(
            filters.len(),
            57,
            "Expected exactly 57 built-in filters, got {}. \
             Update this count when adding/removing filters in src/filters/.",
            filters.len()
        );
    }

    /// Verify that every built-in filter has at least one inline test.
    /// Prevents shipping filters with zero test coverage.
    #[test]
    fn test_builtin_all_filters_have_inline_tests() {
        let mut all_names: Vec<String> = Vec::new();
        let mut tested: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut outcomes = Vec::new();
        collect_test_outcomes(
            BUILTIN_TOML,
            None,
            &mut outcomes,
            &mut all_names,
            &mut tested,
        );

        let untested: Vec<&str> = all_names
            .iter()
            .filter(|name| !tested.contains(name.as_str()))
            .map(|s| s.as_str())
            .collect();

        assert!(
            untested.is_empty(),
            "The following built-in filters have no inline tests: {:?}\n\
             Add [[tests.<name>]] entries to the corresponding src/filters/<name>.toml file.",
            untested
        );
    }

    /// Verify that adding a new filter entry to any TOML content makes it
    /// immediately discoverable via find_filter_in — simulating how a new
    /// src/filters/my-tool.toml would work after cargo build.
    #[test]
    fn test_new_filter_discoverable_after_concat() {
        // Simulate build.rs: concat BUILTIN_TOML with a brand-new filter block
        let new_filter = r#"
[filters.my-new-tool]
description = "Compact my-new-tool output"
match_command = "^my-new-tool\\b"
strip_lines_matching = ["^\\s*$"]
max_lines = 30
on_empty = "my-new-tool: ok"

[[tests.my-new-tool]]
name = "strips blank lines"
input = "output line 1\n\noutput line 2"
expected = "output line 1\noutput line 2"
"#;
        let combined = format!("{}\n\n{}", BUILTIN_TOML, new_filter);
        let filters = make_filters(&combined);

        // All 57 existing filters still present + 1 new = 58
        assert_eq!(
            filters.len(),
            58,
            "Expected 58 filters after concat (57 built-in + 1 new)"
        );

        // New filter is discoverable
        let found = find_filter_in("my-new-tool --verbose", &filters);
        assert!(
            found.is_some(),
            "Newly added filter must be discoverable via find_filter_in"
        );
        assert_eq!(found.unwrap().name, "my-new-tool");
    }

    // --- Track 5: env var substitution + per-platform filters ---

    #[test]
    fn expand_env_substitutes_known_var() {
        let lookup = |k: &str| -> Option<String> {
            match k {
                "MY_HOME" => Some("/home/jidong".into()),
                _ => None,
            }
        };
        assert_eq!(expand_env_vars_with("path: $MY_HOME/bin", lookup), "path: /home/jidong/bin");
    }

    #[test]
    fn expand_env_supports_braced_form() {
        let lookup = |k: &str| -> Option<String> {
            if k == "X" {
                Some("Y".into())
            } else {
                None
            }
        };
        assert_eq!(expand_env_vars_with("a${X}b", lookup), "aYb");
    }

    #[test]
    fn expand_env_silent_on_undefined() {
        let lookup = |_: &str| -> Option<String> { None };
        assert_eq!(expand_env_vars_with("$NOPE/path", lookup), "/path");
    }

    #[test]
    fn expand_env_does_not_eat_regex_backrefs() {
        // Regex backrefs use lowercase digits like $1, $2 — these MUST NOT be
        // captured as env vars or `replace.replacement` strings break.
        let lookup = |_: &str| Some("WRONG".into());
        assert_eq!(expand_env_vars_with("$1 (group)", lookup), "$1 (group)");
        assert_eq!(expand_env_vars_with("$mixedCase", lookup), "$mixedCase");
    }

    #[test]
    fn platform_filter_loads_when_matching_current_os() {
        let toml = format!(
            r#"
schema_version = 1
[filters.platform-test]
match_command = "^xxx"
platform = "{}"
on_empty = "ok"
"#,
            current_os()
        );
        let filters = make_filters(&toml);
        assert_eq!(filters.len(), 1, "filter for current OS should load");
    }

    #[test]
    fn platform_filter_skipped_when_os_mismatches() {
        let other = if current_os() == "linux" {
            "windows"
        } else {
            "linux"
        };
        let toml = format!(
            r#"
schema_version = 1
[filters.platform-test]
match_command = "^xxx"
platform = "{}"
"#,
            other
        );
        let filters = make_filters(&toml);
        assert!(filters.is_empty(), "filter for {} should not load on {}", other, current_os());
    }

    #[test]
    fn platform_filter_unix_loads_on_macos_or_linux() {
        let toml = r#"
schema_version = 1
[filters.unix-test]
match_command = "^xxx"
platform = "unix"
on_empty = "ok"
"#;
        let filters = make_filters(toml);
        if current_os() == "macos" || current_os() == "linux" {
            assert_eq!(filters.len(), 1);
        } else {
            assert!(filters.is_empty());
        }
    }

    #[test]
    fn platform_filter_unset_loads_everywhere() {
        let toml = r#"
schema_version = 1
[filters.no-platform]
match_command = "^xxx"
on_empty = "ok"
"#;
        let filters = make_filters(toml);
        assert_eq!(filters.len(), 1, "platform-less filter must always load");
    }

    #[test]
    fn env_var_substitution_propagates_into_match_command() {
        // SAFETY: we set a unique env var to avoid clobbering anything real.
        // SAFETY: std::env::set_var is safe in single-threaded test execution
        std::env::set_var("CTXZIP_TEST_TOOL", "mytool");
        let toml = r#"
schema_version = 1
[filters.envtest]
match_command = "^$CTXZIP_TEST_TOOL\\b"
on_empty = "envtest: ok"
"#;
        let filters = make_filters(toml);
        // Verify the regex actually matches the substituted command name.
        let result = find_filter_in("mytool --version", &filters);
        assert!(
            result.is_some(),
            "env-substituted match_command must match real input"
        );
        std::env::remove_var("CTXZIP_TEST_TOOL");
    }
}
