# ContextZip Roadmap

Active design: [`docs/superpowers/specs/2026-04-17-contextzip-advancement-design.md`](docs/superpowers/specs/2026-04-17-contextzip-advancement-design.md)

Empirical baseline: 6,850 assistant messages across 10 large Claude Code sessions, **85.8% of context = tool inputs + tool results**. ContextZip currently touches only the Bash slice. Five tracks address the gap.

## Track 1 — Upstream Catch-Up (rtk 0.30.1 → 0.36.0)

Manual logic patches (hash-level cherry-pick infeasible due to RTK→ContextZip rename).

- `git`: `--` separator clap conflict, `-u` short-alias removal, `Stdio::inherit()` for SSH commit signing
- `runner`/`main`: SIGINT/SIGTERM child cleanup
- `cargo`: clippy full error blocks
- `aws`: 8 → 25 subcommands (CloudWatch / Lambda / IAM / DynamoDB / ECS / EC2 SG / S3API / EKS / SQS / Secrets)
- `init`: `--copilot` flag
- `find`: dotfile pattern support
- `curl`: skip JSON schema for localhost
- `read`: default no-filtering + binary detection
- Security: device-hash salt, 0600 salt-file perms

**Skipped (already merged in commit 9ec3790)**: tee UTF-8 truncation, pytest `-q`, pnpm list, Go error detection, RTK env var compat.

## Track 2 — Stability

- Tests for `env_cmd.rs`, `verify_cmd.rs`, `wget_cmd.rs` (security-relevant, currently uncovered)
- `summary.rs:294` cosmetic `Regex::new` → `lazy_static!`

(Independent verification dropped 4 of 7 originally-claimed audit findings as false positives.)

## Track 3 — New Filters

| Filter | Effort | Savings |
|---|---|---|
| `uv` (Python) | 3-4h | 70-80% |
| `gradle` / `mvn` (JVM) | 6-8h | 75-85% |
| `mise` (version mgr) | 3-4h | 60-70% |
| `helm` (K8s) | 6-8h | 65-75% |
| `terraform` (IaC) | 8-12h | 70-80% |
| `biome` (JS/TS lint) | 2-3h | 60-70% |

Skipped: `bun`, `deno`, `rspec`, `oxlint` (low adoption / sub-60% savings).

## Track 4 — Context-History Compressor (NEW)

The first compressor that operates on **past** tool history, not just live stdout.

```bash
contextzip compact <session-id>   # writes <session>.jsonl.compressed sidecar
contextzip apply   <session-id>   # atomic swap with .bak backup
contextzip expand  <session-id>   # rollback
```

v1 ships **two** safe axes only:
- **BashHistoryCompact** — re-apply ContextZip filters to past Bash tool results (idempotent)
- **ReadDedup** — replace repeated reads of the same file with references; expand back if file hash changes (stale-reference protection)

Honest target: **8-10% additional token reduction** on real resumed sessions, fully reversible. Original JSONL never modified.

Dropped from v1 (resume-safety failures): WritePlaceholder, EditDelta, AgentPromptStrip. PreCompact hook integration deferred to v2.

## Track 5 — DSL Polish (demand-gated)

Only adding what users hit:
- Environment variable substitution in patterns (`$HOME`, `$CI_*`)
- Per-platform filter selection (`[filters.cmd.macos]`)

Dropped: `switch` branching, multiline regex, custom function hooks (no demand evidence; existing `unless` covers branching).

## Execution Order

`Track 2 → Track 1 → Track 4 MVP → Track 3 → Track 5` (~11-14 working days)

---

Old RTK roadmap (stability + Homebrew + early adoption) is shipped — see release history.
