# ContextZip Advancement Design

**Date**: 2026-04-17
**Author**: jidong (Claude-assisted)
**Status**: Approved (post-verification)

## Problem

ContextZip is a Rust CLI proxy (fork of `rtk-ai/rtk` at v0.30.1) that filters command output before LLMs see it. Three gaps motivate this work:

1. **Stale upstream**: rtk is at v0.36.0; we are 6 minor versions behind. Bug fixes and AWS coverage have accumulated.
2. **Coverage gaps in 2026 toolchain**: `uv`, `gradle/mvn`, `helm`, `terraform`, `biome`, `mise` have no filters.
3. **Single-axis compression**: ContextZip only compresses real-time stdout. Empirical analysis of 6,850 assistant messages across 10 large sessions shows **tool inputs (46.4%) + tool results (39.4%) = 85.8% of context**. ContextZip touches only the Bash slice of that.

## Goals

- Bring upstream stability and security fixes into ContextZip.
- Expand filter coverage for 2026 toolchain with high-ROI additions only.
- Add a **safe, reversible** context-history compression layer that operates on Claude Code's session JSONL — opt-in, sidecar-based, never destructive.
- Honest token-savings claims backed by measurement, not estimation.

## Non-Goals

- Full upstream merge (RTK→ContextZip rename makes hash-level cherry-pick infeasible).
- DSL feature speculation. Add only on demand evidence.
- In-place JSONL mutation via PreCompact hook (resume-safety risk too high).
- Token-counting precision via `tiktoken-rs` (50-100ms startup violates <10ms target).

## Empirical Baseline

10 largest sessions in `~/.claude/projects/-Users-jidong/`, 6,850 assistant messages, 7.88M chars:

| Layer | Share |
|---|---|
| Tool inputs (Edit/Write/Bash/Read/Agent args) | 46.4% |
| Tool results (Read/Bash/Agent outputs) | 39.4% |
| User text | 10.1% |
| Assistant text | 4.1% |

Top tools by combined footprint: **Read 22.1%, Agent 20.0%, Write 18.9%, Edit 15.8%, Bash 15.0%** (= 91.8%).

Assistant noise patterns (narration, apology, markdown deco) measured at <0.5%. Compressing Claude's own output is **not** the right target.

---

## Design — 5 Tracks

### Track 1: Upstream Cherry-Pick (manual patch)

**Mechanism**: Hash-level cherry-pick fails because of the RTK→ContextZip rename. Instead, manually translate logic from rtk-ai/rtk changelog into ContextZip idioms. Add `upstream` remote for reference only.

**Already merged locally** (verified, skip): tee UTF-8 panic, pytest `-q` summary, pnpm list, Go error detection, RTK env var compat.

**To apply**:
- `git.rs`: `--` separator clap conflict (re-insert after clap consumes from `git diff` args)
- `git.rs`: remove `-u` short alias for `--ultra-compact` (collides with `git push -u`)
- `git.rs`: `Stdio::inherit()` for stdin (preserves SSH commit signing)
- `runner.rs` / `main.rs`: SIGINT/SIGTERM child cleanup (prevent orphaned subprocesses)
- `cargo_cmd.rs`: clippy full error blocks instead of truncated headlines
- `aws_cmd.rs`: expand 8 → 25 subcommands (CloudWatch Logs, CloudFormation, Lambda, IAM, DynamoDB w/ type unwrapping, ECS, EC2 SG, S3API, S3 sync/cp, EKS, SQS, Secrets Manager)
- `init.rs`: `--copilot` flag for GitHub Copilot integration
- `find_cmd.rs`: include hidden files when pattern targets dotfiles
- `curl_cmd.rs`: skip JSON schema for localhost/internal URLs
- `read.rs`: default no-filtering + binary file detection
- **Security**: device-hash salt + 0600 salt-file permissions (verify if not already local)

**Defer to v0.32+**: telemetry 7-bug-fix bundle, hook_check missing-integration detection, hook confirmation prompts.

### Track 2: Audit Punch-List (verified-true items only)

Independent verification confirmed 4 of 7 original audit findings were false positives:

- `playwright_cmd.rs` `.unwrap()` calls — all inside `lazy_static!` blocks (CLAUDE.md-permitted pattern). No fix.
- `hook_audit_cmd.rs:184,193` — inside `#[cfg(test)]` block. No fix.
- `init.rs` `/Users/test/` paths — test fixtures, not prod. No fix.
- `gain.rs:699-743` silent error swallow — intentional graceful degradation for optional bypass detection. No fix.

**Real items**:
1. Add tests to `env_cmd.rs`, `verify_cmd.rs`, `wget_cmd.rs` (security-relevant modules with no test coverage today).
2. Refactor `summary.rs:294` `Regex::new(&format!(...))` to `lazy_static!` (cosmetic correctness; not a hot path).

### Track 3: New Filters (revised)

**Add (5 filters)**:
| Filter | Effort | Savings | Notes |
|---|---|---|---|
| **uv** (Python pkg manager) | 3-4h | 70-80% | Highest ROI for 2026; copy from `pip_cmd.rs` |
| **gradle/mvn** (JVM build) | 6-8h | 75-85% | Enterprise demand; complex build-graph parsing |
| **mise** (version manager) | 3-4h | 60-70% | Replaces nvm/pyenv |
| **helm** (K8s templates) | 6-8h | 65-75% | ⚠️ YAML indentation must be preserved (test rigorously) |
| **terraform** (HCL plan/apply) | 8-12h | 70-80% | ⚠️ Must distinguish "no changes" from real diff |
| **biome** (JS/TS linter) | 2-3h | 60-70% | Trivial copy from eslint pattern |

**Drop**: bun (<5% prod), deno (<60% savings), rspec (Ruby declining), oxlint (unproven adoption).

**Verify**: kubectl is **already complete** in `src/container.rs:230+` (pods/services/logs); plan-claim of "stub routing" was wrong. No work.

### Track 4: Context-History Compressor (NEW, scoped down)

The most valuable but riskiest track. Verification revealed the original "5-axis 91.8%→35%" claim was unrealistic. Revised scope: **safe, opt-in, reversible**.

**Mechanism (explicit, two-step)**:
- New CLI: `contextzip compact --session=<session-id>`
  - Reads `~/.claude/projects/<project>/<session-id>.jsonl`
  - Writes `<session-id>.jsonl.compressed` **as a sidecar** — original untouched
  - Prints summary: bytes saved, refs created, refs that would expand-on-mismatch
- New CLI: `contextzip apply <session-id>` (opt-in atomic swap)
  - Backs up original to `<session-id>.jsonl.bak`
  - Renames `.compressed` to `.jsonl`
  - This is the file Claude Code reads on next session resume
- New CLI: `contextzip expand <session-id>` (rollback)
  - Restores from `.bak` if present, else expands compressed refs back to full content using `first_read_uuid` payloads
- **NOT** wired into PreCompact hook in v1 (in-place mutation during a live session breaks `parentUuid` chain → resume failures). v2 only after sidecar version proves stable.

**Compressors (only 2 of original 5 ship in v1)**:
1. **BashHistoryCompact** — Re-apply existing ContextZip filters to past Bash tool results. Idempotent and safe.
2. **ReadDedup with file-hash validation** — When the same file path is read N times, keep the first read in full and replace subsequent reads with a reference. Hash the file at dedup time; on resume, if file hash differs, expand back to full content (stale-reference protection).

**Dropped axes** (resume-safety failures):
- WritePlaceholder — file may be deleted/moved later; SHA cannot reconstruct.
- EditDelta — collapses intermediate states Claude may need for backward reasoning.
- AgentPromptStrip — boilerplate detection is fragile; false positives drop semantically critical context.

**Honest savings target**: 8-10% reduction on real resumed sessions, fully validated, fully reversible.

**Validation method**: Record N real sessions without compression. Re-run same task with compression. Measure (a) actual token usage on resume, (b) failure / hallucination rate from stale references. Publish numbers, not estimates.

**Deferred to v2**: PreCompact hook integration (only after sidecar version proves stable on real sessions). The other 3 axes (WritePlaceholder, EditDelta, AgentPromptStrip) re-evaluated only with concrete user-validated demand.

### Track 5: TOML DSL Extensions (deferred)

Verification showed all 4 originally proposed DSL features (`switch`, multiline regex, custom functions, snapshot tests) lacked demand evidence. The current 8-stage DSL with 57 built-in filters covers ~80% of real needs.

**Drop**: switch branching, multiline regex, custom function hooks. Do not add until at least one filter author can demonstrate they cannot express their intent.

**Move to Track 2**: `insta` snapshot tests (test infrastructure, not DSL).

**Add only if low-effort**:
- Environment variable substitution in patterns (`$HOME`, `$CI_*`) — real CI-vs-local use case.
- Per-platform filter selection (`[filters.cmd.macos]`) — real cross-platform output divergence.

These two are additive, schema-compatible, and unblock filter authors.

---

## Architecture Notes

### Module additions
```
src/
├── compact_cmd.rs      ← NEW: `contextzip compact <session>` (Track 4)
├── jsonl_rewriter.rs   ← NEW: BashHistoryCompact + ReadDedup (Track 4)
├── helm_cmd.rs         ← NEW (Track 3)
├── terraform_cmd.rs    ← NEW (Track 3)
├── uv_cmd.rs           ← NEW (Track 3)
├── gradle_cmd.rs       ← NEW (Track 3)
├── biome_cmd.rs        ← NEW (Track 3)
├── mise_cmd.rs         ← NEW (Track 3)
├── aws_cmd.rs          ← MODIFIED: 8 → 25 subcommands (Track 1)
├── git.rs              ← MODIFIED: 3 fixes (Track 1)
├── runner.rs / main.rs ← MODIFIED: signal cleanup (Track 1)
├── cargo_cmd.rs        ← MODIFIED: clippy full errors (Track 1)
├── init.rs             ← MODIFIED: --copilot flag (Track 1)
├── summary.rs          ← MODIFIED: lazy_static fix (Track 2)
├── env_cmd.rs          ← MODIFIED: add tests (Track 2)
├── verify_cmd.rs       ← MODIFIED: add tests (Track 2)
├── wget_cmd.rs         ← MODIFIED: add tests (Track 2)
└── toml_filter.rs      ← MODIFIED: env var subst + platform filters (Track 5)
```

### Track 4 data model
```rust
// New JSONL record types written by jsonl_rewriter
{
  "type": "compressed_read_ref",
  "original_uuid": "<uuid>",
  "file_path": "/abs/path",
  "file_sha256": "<hash>",
  "first_read_uuid": "<uuid of full Read>"
}
```

**Apply-time** (`contextzip apply`):
1. Reads sidecar `.compressed` file
2. For each `compressed_read_ref`, checks `file_sha256` matches current disk
3. If match: keep the reference inline (Claude sees a short marker; reasoning treats it as "same content as msg #N")
4. If mismatch: expand back to full content from `first_read_uuid`'s payload (stale-reference protection)
5. Atomic swap with `.bak` backup

v1 keeps the model simple: rewrite the JSONL once at apply-time, no runtime accessor. If a real Claude Code API for lazy substitution emerges later, v2 can layer on it.

### Failure-mode contracts
- **All filters**: existing fallback pattern — on filter failure, pass through unchanged.
- **Track 4 compactor**: on any safety check failure (hash mismatch, missing parent UUID, malformed JSONL), abort write and leave original untouched.
- **Track 4 sidecar**: never modify original `.jsonl`. Any "rollback" is `rm <session>.jsonl.compressed`.

---

## Execution Order

Verification-driven priority (lowest risk first, highest ROI second):

1. **Track 2** (1 day): tests + cosmetic fix. Stabilizes baseline.
2. **Track 1** (1 day): 5 manual patches + AWS expansion + security. Catches up to upstream.
3. **Track 4 MVP** (3-4 days): `contextzip compact` CLI with BashHistoryCompact + ReadDedup, sidecar only. Validate on 5 real sessions, publish honest numbers.
4. **Track 3** (5-7 days): 5 new filters in priority order — biome (warm-up) → uv → mise → helm → terraform → gradle.
5. **Track 5** (1 day if pursued): env var substitution + per-platform filter selection.

Total realistic effort: **11-14 working days**.

---

## Success Criteria

- All Track 1 fixes applied; CI green; no token-savings regression on existing snapshot tests.
- Track 2: env_cmd / verify_cmd / wget_cmd test coverage ≥1 happy-path + ≥1 edge-case each.
- Track 3: each new filter ships with snapshot test + token-savings test ≥60% on real fixtures.
- Track 4: `contextzip compact` runs on a 50MB real session in <2s, produces sidecar smaller than original. `contextzip apply` performs atomic swap with `.bak` backup. `contextzip expand` round-trips losslessly (apply → expand recovers original byte-for-byte when no source files have changed). Measured ≥8% token reduction on real resumed sessions with **zero** task-failure regressions across 5 sessions.
- Track 5: existing user `*.toml` filters continue to parse unchanged.

## Risks

| Risk | Mitigation |
|---|---|
| Upstream rename conflicts on manual patch | Translate logic, not commits. Treat changelog as spec. |
| Track 4 stale-reference bugs | File-hash validation; expand-on-mismatch; always keep original. |
| Filter false positives in helm/terraform | Mandatory negative-case fixtures (no-changes plan, deeply-nested values). |
| New filters break startup time budget (<10ms) | Lazy-load filter modules; benchmark on each PR. |
| Scope creep in Track 5 | Demand-gated; do not add features without GitHub issue + filter author request. |

## Open Questions

- Track 4 v2: when is PreCompact hook integration safe? Need real-session corpus first.
- Track 1: are the rtk security commits already locally applied? Audit before duplicating.
- Track 3: gradle/mvn requires JVM project for fixtures — borrow from a known OSS repo?
