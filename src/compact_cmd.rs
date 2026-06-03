//! `contextzip compact <session-id-or-path>` CLI.
//!
//! Wraps `jsonl_rewriter::compact_session_file`. Resolves a session-id like
//! `bfb59668-...` to its `.jsonl` under `~/.claude/projects/<project>/`,
//! produces a sidecar `.compressed` file, and prints a one-line summary.
//!
//! The original `.jsonl` is never modified. Rollback is `rm <sidecar>`. Apply
//! / expand commands will be added in a follow-up.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

use crate::jsonl_rewriter;

pub fn run_with_options(target: &str, dry_run: bool, verbose: u8) -> Result<()> {
    let session_path = resolve_session_path(target)?;

    if verbose > 0 {
        eprintln!(
            "contextzip compact{}: {}",
            if dry_run { " (dry-run)" } else { "" },
            session_path.display()
        );
    }

    let stats = if dry_run {
        let raw = std::fs::read_to_string(&session_path)
            .with_context(|| format!("Failed to read session: {}", session_path.display()))?;
        jsonl_rewriter::compact_session_str(&raw).1
    } else {
        let (_sidecar, stats) = jsonl_rewriter::compact_session_file(&session_path)
            .with_context(|| format!("Failed to compact session: {}", session_path.display()))?;
        stats
    };

    print_stats_line(&session_path, &stats, dry_run);
    Ok(())
}

/// Compact every `.jsonl` under `~/.claude/projects/<project>/`. Skips files
/// that already have a sidecar so re-running is idempotent. Reports per-session
/// progress on stderr and aggregates totals at the end.
pub fn run_all_sessions(dry_run: bool, verbose: u8) -> Result<()> {
    let root = projects_root()?;
    if !root.is_dir() {
        bail!(
            "No Claude Code projects directory at {} — nothing to do.",
            root.display()
        );
    }

    let mut total_in = 0usize;
    let mut total_out = 0usize;
    let mut total_dedup = 0usize;
    let mut total_bash = 0usize;
    let mut sessions_done = 0usize;
    let mut sessions_skipped = 0usize;

    for project in
        std::fs::read_dir(&root).with_context(|| format!("Failed to read {}", root.display()))?
    {
        let Ok(project) = project else { continue };
        if !project.path().is_dir() {
            continue;
        }
        for file in std::fs::read_dir(project.path())? {
            let Ok(file) = file else { continue };
            let path = file.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            // Skip if a sidecar already exists — assume the user wants to apply
            // it before re-compacting. Avoids silently overwriting prior runs.
            let sidecar = sidecar_path(&path);
            if !dry_run && sidecar.exists() {
                sessions_skipped += 1;
                if verbose > 0 {
                    eprintln!("skip {} (sidecar exists)", path.display());
                }
                continue;
            }

            match run_with_options(path.to_string_lossy().as_ref(), dry_run, verbose.max(1)) {
                Ok(()) => {
                    sessions_done += 1;
                    // We re-read the file just so the aggregate stats are correct;
                    // good enough for an analytics summary, and acceptable for
                    // batch runs (most sessions are <10 MB).
                    if let Ok(raw) = std::fs::read_to_string(&path) {
                        let s = jsonl_rewriter::compact_session_str(&raw).1;
                        total_in += s.bytes_in;
                        total_out += s.bytes_out;
                        total_dedup += s.read_results_deduped;
                        total_bash += s.bash_results_recompressed;
                    }
                }
                Err(e) => eprintln!("FAIL {}: {}", path.display(), e),
            }
        }
    }

    let pct = if total_in > 0 {
        ((total_in - total_out.min(total_in)) as f64 / total_in as f64) * 100.0
    } else {
        0.0
    };
    println!(
        "all-sessions: {} compacted, {} skipped\n  bytes: {} → {} ({:.1}% saved)\n  axes: ReadDedup={}, BashHistoryCompact={}",
        sessions_done, sessions_skipped, total_in, total_out, pct, total_dedup, total_bash
    );
    Ok(())
}

fn print_stats_line(path: &Path, stats: &jsonl_rewriter::CompactStats, dry_run: bool) {
    println!(
        "{}: {}\n  records: {}, bytes: {} → {} ({:.1}% saved)\n  axes: ReadDedup={}, BashHistoryCompact={}",
        if dry_run { "compact (dry-run)" } else { "compact" },
        path.display(),
        stats.records_written,
        stats.bytes_in,
        stats.bytes_out,
        stats.percent_saved(),
        stats.read_results_deduped,
        stats.bash_results_recompressed,
    );
}

/// Atomic swap: backup `<session>.jsonl` → `<session>.jsonl.bak`, then
/// promote `<session>.jsonl.compressed` to `<session>.jsonl`.
///
/// Refuses to run if the sidecar is missing, if a `.bak` already exists
/// (would lose the previous original), or if the original `.jsonl` is gone.
pub fn run_apply(target: &str, verbose: u8) -> Result<()> {
    let session_path = resolve_session_path(target)?;
    let sidecar = sidecar_path(&session_path);
    let backup = backup_path(&session_path);

    if !sidecar.is_file() {
        bail!(
            "No sidecar at {}. Run `contextzip compact {}` first.",
            sidecar.display(),
            target
        );
    }
    if backup.exists() {
        bail!(
            "Backup already exists at {}. Run `contextzip expand {}` first to roll back, or remove the backup manually.",
            backup.display(),
            target
        );
    }

    if verbose > 0 {
        eprintln!(
            "contextzip apply: {} → {} (backup at {})",
            sidecar.display(),
            session_path.display(),
            backup.display()
        );
    }

    std::fs::rename(&session_path, &backup)
        .with_context(|| format!("Failed to back up original session to {}", backup.display()))?;
    if let Err(e) = std::fs::rename(&sidecar, &session_path) {
        // Roll back the backup so we don't leave the user with no live session.
        let _ = std::fs::rename(&backup, &session_path);
        return Err(e).context("Failed to promote sidecar to live session; backup restored");
    }

    println!(
        "apply: {} now active; original preserved at {}",
        session_path.display(),
        backup.display()
    );
    Ok(())
}

/// Restore the original session by renaming `<session>.jsonl.bak` back to
/// `<session>.jsonl`. The current (compressed) `<session>.jsonl` is moved aside
/// to `<session>.jsonl.compressed` so a future `apply` can re-promote it.
///
/// If no `.bak` exists, errors with a clear instruction — there is nothing
/// to expand from. (A future v2 may reconstruct from `contextzip_compressed`
/// annotations alone.)
pub fn run_expand(target: &str, verbose: u8) -> Result<()> {
    let session_path = resolve_session_path(target)?;
    let backup = backup_path(&session_path);
    let sidecar = sidecar_path(&session_path);

    if !backup.is_file() {
        bail!(
            "No backup at {}. There is nothing to expand from. (`contextzip apply` writes the backup; without one we cannot restore.)",
            backup.display()
        );
    }

    if verbose > 0 {
        eprintln!(
            "contextzip expand: {} → {} (current → {})",
            backup.display(),
            session_path.display(),
            sidecar.display()
        );
    }

    // Move the current (compressed) file aside so we keep a working sidecar.
    if session_path.is_file() {
        std::fs::rename(&session_path, &sidecar).with_context(|| {
            format!(
                "Failed to move current session aside to {}",
                sidecar.display()
            )
        })?;
    }
    std::fs::rename(&backup, &session_path)
        .with_context(|| format!("Failed to restore backup from {}", backup.display()))?;

    println!(
        "expand: original restored at {} (compressed copy preserved at {})",
        session_path.display(),
        sidecar.display()
    );
    Ok(())
}

fn sidecar_path(session: &Path) -> PathBuf {
    let mut p = session.to_path_buf();
    let name = session
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("session.jsonl");
    p.set_file_name(format!("{}.compressed", name));
    p
}

fn backup_path(session: &Path) -> PathBuf {
    let mut p = session.to_path_buf();
    let name = session
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("session.jsonl");
    p.set_file_name(format!("{}.bak", name));
    p
}

/// Accept either an absolute/relative path to a `.jsonl` file or a Claude Code
/// session-id (resolved against `~/.claude/projects/`).
fn resolve_session_path(target: &str) -> Result<PathBuf> {
    let direct = PathBuf::from(target);
    if direct.is_file() {
        return Ok(direct);
    }

    let projects_root = projects_root()?;
    let mut hits: Vec<PathBuf> = Vec::new();
    // A missing/unreadable projects dir simply means there are no sessions to
    // match — fall through to the clear "No session found" error below rather
    // than surfacing a low-level read error (this is also what CI hits, where
    // ~/.claude/projects does not exist).
    if let Ok(entries) = std::fs::read_dir(&projects_root) {
        for project_dir in entries {
            let Ok(project_dir) = project_dir else {
                continue;
            };
            if !project_dir.path().is_dir() {
                continue;
            }
            let candidate = project_dir.path().join(format!("{}.jsonl", target));
            if candidate.is_file() {
                hits.push(candidate);
            }
        }
    }

    match hits.len() {
        0 => bail!(
            "No session found. Tried: {} (file) and `{}.jsonl` under {}",
            target,
            target,
            projects_root.display()
        ),
        1 => Ok(hits.into_iter().next().unwrap()),
        n => bail!(
            "Session id `{}` matched {} files across multiple projects. Pass the full path instead.",
            target,
            n
        ),
    }
}

fn projects_root() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".claude").join("projects"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn resolve_session_path_accepts_existing_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("session.jsonl");
        let mut f = fs::File::create(&file).unwrap();
        writeln!(f, "{{\"type\":\"user\"}}").unwrap();
        let resolved = resolve_session_path(file.to_str().unwrap()).unwrap();
        assert_eq!(resolved, file);
    }

    #[test]
    fn resolve_session_path_errors_clearly_when_missing() {
        let r = resolve_session_path("__definitely_no_such_session_id_zzzz__");
        assert!(r.is_err());
        let msg = format!("{}", r.unwrap_err());
        assert!(
            msg.contains("No session found"),
            "expected 'No session found' in: {}",
            msg
        );
    }

    #[test]
    fn run_compacts_a_real_jsonl_file() -> Result<()> {
        let dir = TempDir::new()?;
        let session = dir.path().join("session.jsonl");
        let mut f = fs::File::create(&session)?;
        // A trivial Bash result that should pass through (too small to compress).
        writeln!(
            f,
            r#"{{"type":"assistant","uuid":"a1","message":{{"content":[{{"type":"tool_use","id":"b1","name":"Bash","input":{{"command":"true"}}}}]}}}}"#
        )?;
        writeln!(
            f,
            r#"{{"type":"user","uuid":"u1","message":{{"content":[{{"type":"tool_result","tool_use_id":"b1","content":"ok\n"}}]}}}}"#
        )?;
        drop(f);

        // Should not panic, should produce a sidecar.
        run_with_options(session.to_str().unwrap(), false, 0)?;
        let sidecar = dir.path().join("session.jsonl.compressed");
        assert!(
            sidecar.exists(),
            "expected sidecar at {}",
            sidecar.display()
        );
        Ok(())
    }

    #[test]
    fn dry_run_does_not_write_sidecar() -> Result<()> {
        let dir = TempDir::new()?;
        let session = dir.path().join("session.jsonl");
        fs::write(&session, "{}\n")?;
        run_with_options(session.to_str().unwrap(), true, 0)?;
        let sidecar = dir.path().join("session.jsonl.compressed");
        assert!(
            !sidecar.exists(),
            "dry-run must NOT write sidecar (found one at {})",
            sidecar.display()
        );
        Ok(())
    }

    fn make_repeatable_session(dir: &Path) -> Result<PathBuf> {
        let session = dir.join("session.jsonl");
        let mut f = fs::File::create(&session)?;
        // Two reads of the same file → ReadDedup will trigger.
        writeln!(
            f,
            r#"{{"type":"assistant","uuid":"a1","message":{{"content":[{{"type":"tool_use","id":"r1","name":"Read","input":{{"file_path":"/tmp/x.rs"}}}}]}}}}"#
        )?;
        writeln!(
            f,
            r#"{{"type":"user","uuid":"u1","message":{{"content":[{{"type":"tool_result","tool_use_id":"r1","content":"fn main() {{}}"}}]}}}}"#
        )?;
        writeln!(
            f,
            r#"{{"type":"assistant","uuid":"a2","message":{{"content":[{{"type":"tool_use","id":"r2","name":"Read","input":{{"file_path":"/tmp/x.rs"}}}}]}}}}"#
        )?;
        writeln!(
            f,
            r#"{{"type":"user","uuid":"u2","message":{{"content":[{{"type":"tool_result","tool_use_id":"r2","content":"fn main() {{}}"}}]}}}}"#
        )?;
        Ok(session)
    }

    #[test]
    fn apply_swaps_sidecar_into_place_and_creates_backup() -> Result<()> {
        let dir = TempDir::new()?;
        let session = make_repeatable_session(dir.path())?;
        run_with_options(session.to_str().unwrap(), false, 0)?;
        let original_bytes = fs::read(&session)?;

        run_apply(session.to_str().unwrap(), 0)?;

        let backup = dir.path().join("session.jsonl.bak");
        let sidecar = dir.path().join("session.jsonl.compressed");
        assert!(backup.is_file(), "backup should exist after apply");
        assert!(!sidecar.exists(), "sidecar should be consumed by apply");
        assert_eq!(
            fs::read(&backup)?,
            original_bytes,
            "backup must equal pre-apply content"
        );
        Ok(())
    }

    #[test]
    fn apply_refuses_when_no_sidecar() -> Result<()> {
        let dir = TempDir::new()?;
        let session = dir.path().join("session.jsonl");
        fs::write(&session, "{}\n")?;
        let r = run_apply(session.to_str().unwrap(), 0);
        assert!(r.is_err());
        let msg = format!("{}", r.unwrap_err());
        assert!(msg.contains("No sidecar"), "wrong error: {}", msg);
        Ok(())
    }

    #[test]
    fn apply_refuses_when_backup_already_present() -> Result<()> {
        let dir = TempDir::new()?;
        let session = make_repeatable_session(dir.path())?;
        run_with_options(session.to_str().unwrap(), false, 0)?;
        // Pre-create a backup to simulate a prior apply.
        fs::write(dir.path().join("session.jsonl.bak"), "old backup")?;
        let r = run_apply(session.to_str().unwrap(), 0);
        assert!(r.is_err());
        assert!(format!("{}", r.unwrap_err()).contains("Backup already exists"));
        Ok(())
    }

    #[test]
    fn expand_restores_original_byte_for_byte() -> Result<()> {
        let dir = TempDir::new()?;
        let session = make_repeatable_session(dir.path())?;
        let original_bytes = fs::read(&session)?;

        run_with_options(session.to_str().unwrap(), false, 0)?;
        run_apply(session.to_str().unwrap(), 0)?;
        run_expand(session.to_str().unwrap(), 0)?;

        let restored = fs::read(&session)?;
        assert_eq!(
            restored, original_bytes,
            "expand must round-trip apply losslessly"
        );
        // The compressed copy is preserved on the side so apply can be redone.
        let sidecar = dir.path().join("session.jsonl.compressed");
        assert!(sidecar.is_file());
        Ok(())
    }

    #[test]
    fn expand_errors_clearly_when_no_backup() -> Result<()> {
        let dir = TempDir::new()?;
        let session = dir.path().join("session.jsonl");
        fs::write(&session, "{}\n")?;
        let r = run_expand(session.to_str().unwrap(), 0);
        assert!(r.is_err());
        assert!(format!("{}", r.unwrap_err()).contains("No backup"));
        Ok(())
    }
}
