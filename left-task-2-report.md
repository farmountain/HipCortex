# Left-task-2 report: Product docs

**Date:** 2026-07-18  
**Cwd:** `D:\all_projects\HipCortex`  
**Commit message:** `docs: proactive harness, usage, and agent gitnexus rules`  
**Push:** no

## Shipped

| Path | Action | Notes |
|------|--------|--------|
| `README.md` | updated | Proactive harness section: install cmd, MUST query table, ~93% token path, link to usage |
| `docs/usage.md` | updated | Feature-gated build/test/bench cmds; substrate-first loop; reflect curl; surface table |
| `docs/launch/show-hn.md` | updated | Restored submit URL; latency matrix; proactive harness bullet; live_beliefs |
| `CLAUDE.md` | updated | GitNexus stats → **6888** symbols, **13530** relationships, **300** flows |
| `AGENTS.md` | added | Project GitNexus rules (same block as CLAUDE.md); HipCortex-specific |
| `TEST_VALIDATION_REPORT.md` | updated | 2026-07-17 verification of sdk-tmf / intelligence-foundation / agent-substrate-autonomy |
| `HipCortex_E2E_User_Testing_Plan.md` | added | Structured E2E harness plan (v0.4.9 dated plan; quality OK) |

## Skipped

| Path | Reason |
|------|--------|
| `CLAUDE.gitnexus.md` | Duplicate of `AGENTS.md` with stale stats (2876/5788/123). Prefer single `AGENTS.md` |
| `.claude/skills/` | Excluded (agent marketplace junk; not project-owned product docs) |

## GitNexus index (verified)

From `gitnexus list_repos` / HipCortex index (`2026-07-18`):

- **nodes (symbols):** 6888  
- **edges (relationships):** 13530  
- **processes:** 300  
- **files:** 555  

Stats written into `CLAUDE.md` + `AGENTS.md`. Index ~2 commits behind HEAD at report time (docs-only; re-analyze optional).

## Content summary

**Proactive harness (product surface):**

```bash
hipcortex install --mode proactive --actor my_project_agent
```

- MUST `GET /memory/live_beliefs` (or search) before frontier tokens  
- Offload hyp / world model / self-health to Rust substrate  
- `POST /memory/reflect` + `POST /decide/can-execute` + `/memory/ingest`  
- Token path: proactive substrate ~93% steady-state savings vs full history  
- Template: `sdk/python/hipcortex/install/SKILL.md`

## Not in this commit

- `.superpowers/sdd/progress.md` (task tracking only)  
- VSIX binaries, adapter scripts, GitHub workflows, untracked build artifacts  
- Code / feature changes (docs-only)

## Verification

- [x] No `.claude/skills/` in commit  
- [x] No push  
- [x] Product paths listed above staged and committed  
- [x] GitNexus stats match live index  
