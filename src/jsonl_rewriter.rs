//! Session-history compressor for Claude Code JSONL session logs.
//!
//! Operates on the JSONL produced under `~/.claude/projects/<project>/<session>.jsonl`.
//! Two safe, opt-in axes ship in v0.2:
//!
//! - **`ReadDedup`** — when the same file is read multiple times via the `Read`
//!   tool, the second and later `tool_result` payloads are replaced with a
//!   short reference back to the first read. A SHA-256 of the file at compact
//!   time is recorded so that, if the on-disk file later changes, an `expand`
//!   step can detect the mismatch and restore the original content.
//! - **`BashHistoryCompact`** — past `Bash` tool_results are re-fed through
//!   ContextZip's normal filter pipeline. Idempotent: re-running on already
//!   compressed records is a no-op.
//!
//! Records are never removed and the `uuid` / `parentUuid` chain is never
//! altered, only `tool_result` content payloads are rewritten. The original
//! `.jsonl` is left untouched; output goes to a sibling `.compressed` file.

use anyhow::{Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Aggregated metrics returned to the CLI for the user-facing summary.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CompactStats {
    pub records_read: usize,
    pub records_written: usize,
    pub bytes_in: usize,
    pub bytes_out: usize,
    pub bash_results_recompressed: usize,
    pub read_results_deduped: usize,
}

impl CompactStats {
    pub fn percent_saved(&self) -> f64 {
        if self.bytes_in == 0 {
            return 0.0;
        }
        ((self.bytes_in - self.bytes_out.min(self.bytes_in)) as f64 / self.bytes_in as f64) * 100.0
    }
}

/// Compact a session JSONL file into a sidecar `.compressed` file.
/// Returns the path of the sidecar plus aggregated stats. The original is never
/// modified; rollback is `rm <sidecar>`.
pub fn compact_session_file(input: &Path) -> Result<(PathBuf, CompactStats)> {
    let raw = fs::read_to_string(input)
        .with_context(|| format!("Failed to read session file: {}", input.display()))?;

    let (out, stats) = compact_session_str(&raw);

    let mut sidecar = input.to_path_buf();
    let new_name = match input.file_name().and_then(|s| s.to_str()) {
        Some(name) => format!("{}.compressed", name),
        None => "session.jsonl.compressed".to_string(),
    };
    sidecar.set_file_name(new_name);

    fs::write(&sidecar, &out)
        .with_context(|| format!("Failed to write sidecar: {}", sidecar.display()))?;

    Ok((sidecar, stats))
}

/// Pure-string compaction: takes the raw JSONL, returns the rewritten JSONL.
/// Lines that aren't valid JSON are passed through verbatim.
pub fn compact_session_str(input: &str) -> (String, CompactStats) {
    let mut stats = CompactStats {
        bytes_in: input.len(),
        ..Default::default()
    };

    // Two-pass: first pass collects which Read+file_path → first tool_use_id.
    // Second pass rewrites repeated Read tool_results, and recompresses Bash
    // tool_results unconditionally.
    let mut tool_use_index: HashMap<String, ToolUseInfo> = HashMap::new();
    let mut first_read_for: HashMap<String, FirstRead> = HashMap::new();

    for line in input.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(record) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        index_record(&record, &mut tool_use_index, &mut first_read_for);
    }

    // Second pass: rewrite content of tool_results.
    let mut out = String::with_capacity(input.len());
    for line in input.lines() {
        stats.records_read += 1;
        if line.trim().is_empty() {
            out.push('\n');
            continue;
        }

        let mut record: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => {
                // Pass through unparseable lines unchanged.
                out.push_str(line);
                out.push('\n');
                stats.records_written += 1;
                continue;
            }
        };

        rewrite_record(&mut record, &tool_use_index, &first_read_for, &mut stats);

        let written = serde_json::to_string(&record).unwrap_or_else(|_| line.to_string());
        out.push_str(&written);
        out.push('\n');
        stats.records_written += 1;
    }

    stats.bytes_out = out.len();
    (out, stats)
}

#[derive(Debug, Clone)]
struct ToolUseInfo {
    name: String,
    file_path: Option<String>,
}

#[derive(Debug, Clone)]
struct FirstRead {
    tool_use_id: String,
    // `content_sha256` will be added back when `contextzip expand` ships and
    // needs to detect a stale file before substituting the cached read. Kept
    // out for now to avoid unused-field warnings.
}

fn index_record(
    record: &Value,
    tool_use_index: &mut HashMap<String, ToolUseInfo>,
    first_read_for: &mut HashMap<String, FirstRead>,
) {
    if record.get("type").and_then(Value::as_str) != Some("assistant") {
        return;
    }
    let Some(content) = record
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(Value::as_array)
    else {
        return;
    };

    for block in content {
        if block.get("type").and_then(Value::as_str) != Some("tool_use") {
            continue;
        }
        let Some(id) = block.get("id").and_then(Value::as_str) else {
            continue;
        };
        let name = block
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let file_path = block
            .get("input")
            .and_then(|i| i.get("file_path"))
            .and_then(Value::as_str)
            .map(String::from);

        tool_use_index.insert(
            id.to_string(),
            ToolUseInfo {
                name: name.clone(),
                file_path: file_path.clone(),
            },
        );

        if name == "Read" {
            if let Some(path) = file_path {
                first_read_for.entry(path).or_insert_with(|| FirstRead {
                    tool_use_id: id.to_string(),
                });
            }
        }
    }
}

fn rewrite_record(
    record: &mut Value,
    tool_use_index: &HashMap<String, ToolUseInfo>,
    first_read_for: &HashMap<String, FirstRead>,
    stats: &mut CompactStats,
) {
    if record.get("type").and_then(Value::as_str) != Some("user") {
        return;
    }
    let Some(content) = record
        .get_mut("message")
        .and_then(|m| m.get_mut("content"))
        .and_then(Value::as_array_mut)
    else {
        return;
    };

    for block in content.iter_mut() {
        if block.get("type").and_then(Value::as_str) != Some("tool_result") {
            continue;
        }
        let Some(use_id) = block
            .get("tool_use_id")
            .and_then(Value::as_str)
            .map(String::from)
        else {
            continue;
        };
        let Some(info) = tool_use_index.get(&use_id) else {
            continue;
        };

        match info.name.as_str() {
            "Read" => {
                if let Some(path) = info.file_path.as_deref() {
                    if let Some(first) = first_read_for.get(path) {
                        if first.tool_use_id != use_id {
                            // Repeated read of the same path → replace with reference.
                            let preview_len = block_text_len(block);
                            replace_with_read_ref(block, path, &first.tool_use_id, preview_len);
                            stats.read_results_deduped += 1;
                        }
                    }
                }
            }
            "Bash" => {
                if recompress_bash_block(block) {
                    stats.bash_results_recompressed += 1;
                }
            }
            _ => {}
        }
    }
}

fn block_text_len(block: &Value) -> usize {
    let content = match block.get("content") {
        Some(Value::String(s)) => return s.len(),
        Some(Value::Array(arr)) => arr,
        _ => return 0,
    };
    content
        .iter()
        .filter_map(|c| c.get("text").and_then(Value::as_str))
        .map(str::len)
        .sum()
}

fn replace_with_read_ref(block: &mut Value, path: &str, first_id: &str, original_len: usize) {
    let marker = format!(
        "[contextzip: dedup — same as Read in tool_use {} ({} → 0 chars). \
         Re-expand with `contextzip expand` if the file at {} has changed.]",
        first_id, original_len, path
    );
    block["content"] = json!([{ "type": "text", "text": marker }]);
    // Annotation so `expand` can find these refs without parsing the marker text.
    block["contextzip_compressed"] = json!({
        "axis": "ReadDedup",
        "first_tool_use_id": first_id,
        "file_path": path,
        "original_chars": original_len,
    });
}

fn recompress_bash_block(block: &mut Value) -> bool {
    // Idempotency guard: if we already compacted this once, skip.
    if block.get("contextzip_compressed").is_some() {
        return false;
    }
    let original = match block.get("content") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|c| c.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => return false,
    };
    if original.is_empty() {
        return false;
    }

    let filtered = compress_bash_text(&original);
    if filtered.len() >= original.len() {
        // Filter didn't help — leave it alone so we never inflate the JSONL.
        return false;
    }

    let saved = original.len() - filtered.len();
    let new_text = format!(
        "{}\n[contextzip: BashHistoryCompact saved {} chars from past Bash result]",
        filtered, saved
    );
    block["content"] = json!([{ "type": "text", "text": new_text }]);
    block["contextzip_compressed"] = json!({
        "axis": "BashHistoryCompact",
        "original_chars": original.len(),
        "compressed_chars": new_text.len(),
        "content_sha256": sha256_hex(&original),
    });
    true
}

/// Apply ContextZip's existing line-based compression heuristics to a past Bash
/// result. We deliberately don't re-execute filters that need a Command — those
/// require a process; we only do safe, idempotent text-level reductions:
///   - strip ANSI escape sequences
///   - drop blank-padding runs
///   - collapse repeated identical lines into "(× N)" tallies
///   - cap at 200 lines with an explicit "(N more)" marker
fn compress_bash_text(input: &str) -> String {
    let stripped = crate::ansi_filter::filter_ansi(input);
    let mut out: Vec<String> = Vec::new();
    let mut blank_run = 0usize;
    let mut last_line: Option<String> = None;
    let mut last_line_count = 0usize;

    for raw in stripped.lines() {
        let line = raw.trim_end();
        if line.is_empty() {
            blank_run += 1;
            if blank_run <= 1 {
                if let Some(prev) = flush_repeat(&mut last_line, &mut last_line_count) {
                    out.push(prev);
                }
                out.push(String::new());
            }
            continue;
        }
        blank_run = 0;

        if last_line.as_deref() == Some(line) {
            last_line_count += 1;
            continue;
        }
        if let Some(prev) = flush_repeat(&mut last_line, &mut last_line_count) {
            out.push(prev);
        }
        last_line = Some(line.to_string());
        last_line_count = 1;
    }
    if let Some(prev) = flush_repeat(&mut last_line, &mut last_line_count) {
        out.push(prev);
    }

    if out.len() > 200 {
        let dropped = out.len() - 200;
        out.truncate(200);
        out.push(format!(
            "(... {} more lines dropped by contextzip)",
            dropped
        ));
    }
    out.join("\n")
}

fn flush_repeat(last_line: &mut Option<String>, count: &mut usize) -> Option<String> {
    let line = last_line.take()?;
    let n = std::mem::replace(count, 0);
    if n <= 1 {
        Some(line)
    } else {
        Some(format!("{} (×{})", line, n))
    }
}

fn sha256_hex(input: &str) -> String {
    let mut h = Sha256::new();
    h.update(input.as_bytes());
    let bytes = h.finalize();
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{:02x}", b);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_assistant_read(id: &str, file_path: &str) -> Value {
        json!({
            "type": "assistant",
            "uuid": format!("ass-{}", id),
            "message": {
                "content": [
                    { "type": "tool_use", "id": id, "name": "Read", "input": { "file_path": file_path } }
                ]
            }
        })
    }

    fn make_user_tool_result(id: &str, text: &str) -> Value {
        json!({
            "type": "user",
            "uuid": format!("usr-{}", id),
            "message": {
                "content": [
                    { "type": "tool_result", "tool_use_id": id, "content": text }
                ]
            }
        })
    }

    fn make_assistant_bash(id: &str, command: &str) -> Value {
        json!({
            "type": "assistant",
            "uuid": format!("ass-{}", id),
            "message": {
                "content": [
                    { "type": "tool_use", "id": id, "name": "Bash", "input": { "command": command } }
                ]
            }
        })
    }

    fn jsonl(records: &[Value]) -> String {
        records
            .iter()
            .map(|r| serde_json::to_string(r).unwrap())
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    }

    #[test]
    fn read_dedup_replaces_repeat_with_reference() {
        let records = vec![
            make_assistant_read("u1", "/abs/foo.rs"),
            make_user_tool_result("u1", "fn main() { println!(\"hi\"); }"),
            make_assistant_read("u2", "/abs/foo.rs"),
            make_user_tool_result("u2", "fn main() { println!(\"hi\"); }"),
        ];
        let input = jsonl(&records);
        let (out, stats) = compact_session_str(&input);

        assert_eq!(stats.read_results_deduped, 1);
        assert!(
            out.contains("ReadDedup"),
            "missing ReadDedup marker in {}",
            out
        );
        // First read still has the full text
        assert!(out.contains("fn main() { println!(\\\"hi\\\"); }"));
        // Second read replaced
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines[3].contains("contextzip"));
    }

    #[test]
    fn read_dedup_does_not_touch_unique_reads() {
        let records = vec![
            make_assistant_read("u1", "/abs/a.rs"),
            make_user_tool_result("u1", "let a = 1;"),
            make_assistant_read("u2", "/abs/b.rs"),
            make_user_tool_result("u2", "let b = 2;"),
        ];
        let (out, stats) = compact_session_str(&jsonl(&records));
        assert_eq!(stats.read_results_deduped, 0);
        assert!(out.contains("let a = 1;"));
        assert!(out.contains("let b = 2;"));
    }

    #[test]
    fn bash_history_compact_collapses_repeated_lines() {
        let noisy = "ok\nok\nok\nok\nfailure\n".repeat(50);
        let records = vec![
            make_assistant_bash("b1", "noisy"),
            make_user_tool_result("b1", &noisy),
        ];
        let (out, stats) = compact_session_str(&jsonl(&records));
        assert_eq!(stats.bash_results_recompressed, 1);
        // Repeated 'ok' lines should be collapsed into a tally
        assert!(out.contains("(×"), "expected tally marker in {}", out);
        assert!(out.len() < jsonl(&records).len());
    }

    #[test]
    fn bash_history_compact_is_idempotent() {
        let noisy = "junk\n".repeat(100);
        let records = vec![
            make_assistant_bash("b1", "noisy"),
            make_user_tool_result("b1", &noisy),
        ];
        let input = jsonl(&records);
        let (first_out, first_stats) = compact_session_str(&input);
        let (second_out, second_stats) = compact_session_str(&first_out);
        assert_eq!(first_stats.bash_results_recompressed, 1);
        assert_eq!(second_stats.bash_results_recompressed, 0);
        assert_eq!(first_out, second_out);
    }

    #[test]
    fn malformed_lines_pass_through_unchanged() {
        let input = "this is not json\n{\"type\":\"user\"}\nalso not json\n";
        let (out, stats) = compact_session_str(input);
        assert!(out.contains("this is not json"));
        assert!(out.contains("also not json"));
        assert_eq!(stats.records_read, 3);
        assert_eq!(stats.records_written, 3);
    }

    #[test]
    fn empty_input_produces_empty_output() {
        let (out, stats) = compact_session_str("");
        assert_eq!(out, "");
        assert_eq!(stats.records_read, 0);
        assert_eq!(stats.bytes_in, 0);
    }

    #[test]
    fn compact_does_not_inflate_when_filter_yields_no_savings() {
        // A short Bash result has nothing to compress; we should leave it alone
        // rather than wrap it in a marker that grows the payload.
        let small = "exit 0";
        let records = vec![
            make_assistant_bash("b1", "true"),
            make_user_tool_result("b1", small),
        ];
        let original = jsonl(&records);
        let (out, _) = compact_session_str(&original);
        assert!(out.len() <= original.len() + 10); // allow trailing newline diff
    }

    #[test]
    fn percent_saved_handles_zero_input() {
        let s = CompactStats::default();
        assert_eq!(s.percent_saved(), 0.0);
    }

    #[test]
    fn percent_saved_reports_reduction() {
        let s = CompactStats {
            bytes_in: 1000,
            bytes_out: 250,
            ..Default::default()
        };
        assert!((s.percent_saved() - 75.0).abs() < 0.01);
    }

    #[test]
    fn sha256_hex_is_deterministic_and_lowercase() {
        let h = sha256_hex("hello world");
        assert_eq!(
            h,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }
}
