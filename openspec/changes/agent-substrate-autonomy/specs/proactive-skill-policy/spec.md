# Spec: Proactive Skill Policy (Claude Agent Harness)

## Overview
The installed SKILL.md (and CLAUDE.md registration) is the primary policy for Claude Code (and similar) agents using HipCortex. Currently conservative/reactive (from `sdk/python/hipcortex/install/SKILL.md` template, written by `_install_claude_code` in cli.py). This capability makes it substrate-first/proactive to align with memory-centric vision (agent orchestrates substrate as mind; LLM narrow for hyp gen/final; 80-99%+ reduction).

The SKILL + MCP tools + substrate observations (live_beliefs, context, hyp) becomes the explicit "Claude Agent Harness" (per agent-harness-construction: optimized action space for substrate use, observations from substrate, higher completion rates, minimal LLM calls).

## Requirements
- **Policy Text (in template)**: Update from "Invoke HipCortex when the user asks you to" to:
  - "You are a memory-centric agent. The HipCortex substrate (memories + hypotheses graph as persistent beliefs + world model predictions + self health + coherence) is your primary durable mind and reasoning engine."
  - "MUST: Before any question involving project state, architecture, decisions, bugs, or context, call search_memory or get_live_beliefs (or /memory/context) first."
  - "MUST: After any decision, architectural discussion, bug fix, or important finding, use /memory/ingest or add_memory (or reflect for CoT)."
  - "Use worldmodel/*, self/*, coherence/* as reasoning primitives (predict, health, invariants)."
  - "LLM (you) only for final natural language output or creative hypothesis generation when substrate signals high entropy/uncertainty (via reflect or explicit). Offload state, beliefs, prediction, consistency to substrate."
  - "Harness: Action space = MCP tools (/memory/*, /hipcortex) + REST equivalents. Observations = live_beliefs (merged symbolic + hyp + world + intel), context, hyp updates. Goal: maximize substrate use for completion; minimize frontier LLM context/effort (target 80-99%+ reduction per benchmarks)."
- **Examples**: Add 3-5 few-shot in SKILL (e.g., "User asks about DB decision → first /memory/search or live_beliefs → use result → decide/store via ingest").
- **Slash & Auto**: Keep /hipcortex as explicit (for user control/overrides); enhance auto-memory to proactive ingest after decisions.
- **Registration**: CLAUDE.md block (from cli.py) points to harness/SKILL; mentions "Trigger: /hipcortex" but policy drives proactive.
- **Installer**: Supports --mode proactive (writes this version); default conservative for backward. Substitutes server URL.
- **MCP Shim**: Update descriptions/examples to recommend live_beliefs first for harness.
- **Multi-Agent**: Policy notes actor scoping (default git repo); global vs per-actor via installer flag or env.
- **Compliance Notes**: Model follows with good examples + reinforcement; hybrid with occasional user nudges OK. Test in validation.
- **No Breaking**: Conservative text remains as alt/comment; explicit/manual paths (user says "remember") still work.

## Acceptance Criteria
- Template updated (sdk/python/hipcortex/install/SKILL.md); content matches above.
- Installer writes proactive version with --mode; CLAUDE.md registration updated.
- MCP shim examples use harness pattern.
- Agent simulation (manual or test) shows self-calls to substrate first (logs/calls confirm).
- Docs reference harness (README, docs/usage, claude-agent-harness spec).
- Part of overall 80%+ reduction in validation benchmarks.
- GitNexus impact on "install" / "SKILL" flows clean (no breakage).

## Dependencies
- Relies on unified-beliefs-surface (get_live_beliefs) and engine-agent-defaults (for substrate to be rich).
- Feeds claude-agent-harness spec (this *is* the harness policy).
- Validation-benchmarks (measures compliance + reduction).

## Risks & Mitigations
- Model compliance <100% (from exploration thread 3): Mitigation - strong examples in SKILL, tool descs, hybrid nudges, measured in benchmarks.
- Token thrashing from extra calls (thread 1): Mitigation - unified surface (fewer calls), substrate cheap (benchmarks), net win shown.
- User control loss: Mitigation - /hipcortex explicit, conservative default available, pinned memories for rules.
- Python/Rust seam: Installer owns policy; co-version with Rust changes.

This spec resolves the policy gap (conservative SKILL) to enable memory-centric invocation. Surgical (template + installer update). Verifiable.