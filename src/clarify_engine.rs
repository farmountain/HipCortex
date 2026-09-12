//! ClarifyEngine — self-prompting clarity loop for goal problem statements.
//!
//! Chain-of-thought:
//!   Ambiguity is resolved by a **strictly-descending information ladder**, never by
//!   repetition. Each rung is strictly more informative (and more expensive) than the one
//!   above it, and each rung fires **at most once per goal**:
//!
//!     T0 environment restatement — rewrites AC that contradict recent evidence
//!     T1 prior-art restatement   — adopts a decidable AC from the substrate's own past
//!     T2 causal attribution      — names the structural equation that explains the failure
//!     T3 ask the user            — one deduped Belief{clarify_needed}
//!
//!   Why a ladder instead of `MAX_CLARIFY_ROUNDS` (the v3.6 shape): that loop ran the
//!   *same* belief search three times. If round 1 found nothing, rounds 2 and 3 could not
//!   succeed either — the store had not changed between them — so the counter was
//!   decorative and the ladder was really one rung deep. Termination came from the
//!   constant, not from a property of the search. Monotone descent makes termination
//!   structural: three tiers, each consumed once, with the ledger persisted so the bound
//!   survives a restart.
//!
//! Exit guarantees (each is enforced here, and each has a test):
//!   1. **Tier monotonicity.** A rung is persisted as `Reflexion{action:"clarify_tier"}`
//!      keyed by `(goal_id, tier)`. A tier whose rung is already recorded is never
//!      re-entered for that goal.
//!   2. **Ladder exhaustion.** At most MAX_CLARIFY_TIERS rungs; T3 is terminal.
//!   3. **Lifetime budget.** MAX_CLARIFY_CYCLES_PER_GOAL counts *invocations* per goal
//!      (`Reflexion{action:"clarify_cycle"}`). Re-invocation is the real unbounded hazard:
//!      `substrate_daemon` drives goals tick by tick, so a per-invocation bound alone
//!      bounds nothing over time. Exhaustion forces T3 with a terminal status.
//!   4. **No-progress exit.** A descent that closes no new rung exits to T3 in the same
//!      invocation instead of re-running rungs it cannot advance.
//!   5. **Substrate budget.** Each rung debits `SubstrateBudget`; `spend >= b0` forces T3.
//!      `default_b0()` is defined as `MAX_CLARIFY_CYCLES_PER_GOAL * C_MIN` so the budget
//!      and the lifetime cap agree by construction rather than contradicting each other.
//!   6. **Ask-cost gate.** "Only critical questions should clarify with users" is made
//!      computable: T3 asks iff `P(unresolvable) * cost_of_wrong_execution > cost_of_asking`.
//!      When it declines, the substrate keeps its current reading and writes
//!      `Reflexion{clarify_ask_declined}` — no human attention is spent on a question the
//!      substrate judged not worth asking.
//!
//!   `ReactEngine::max_react_iterations` remains the outer bound; the ladder is strictly
//!   inside it.
//!
//! v2.1.0: `restate_if_env_changed` — rewrites success_factors when environment signals
//! make them unachievable (failure keyword × factor keyword overlap in recent Temporal
//! records), then writes `Reflexion{goal_restated}`. Now the T0 rung.

use uuid::Uuid;

/// Number of *search* rungs in the ladder (T0, T1, T2). T3 is terminal, not a rung.
pub const MAX_CLARIFY_TIERS: u32 = 3;

/// Lifetime bound: how many times the ladder may be **invoked** for one goal.
///
/// Invocations, not rounds. A round bound inside one invocation says nothing about a
/// daemon that re-invokes the ladder every tick (C3); this counts the re-invocations.
pub const MAX_CLARIFY_CYCLES_PER_GOAL: u32 = 3;

/// Gate on `AttributionReport` for the T2 rung: the existing `single_intervention_sufficient`
/// threshold is derived from this same number in `CausalGraph::credit_assign`.
pub const CAUSAL_CONFIDENCE_GATE: f64 = 0.85;

/// Minimum success-factor overlap for a prior-art goal to count as evidence (T1).
pub const PRIOR_ART_OVERLAP_GATE: f64 = 0.5;

/// What one unit of user attention costs, in the goal's own `estimated_cost` units.
///
/// Calibrated so the gate does not change behaviour for a default goal: every goal
/// created without an explicit `estimated_cost` carries `1.0`, so the failure path
/// (`P(unresolvable) = 1.0`) asks whenever the goal is not cheaper than a quarter unit.
/// A goal that declares itself cheap *and* whose ladder stalled on unavailability is not
/// worth a human — that is the case this constant exists to refuse.
pub const COST_OF_ASKING: f64 = 0.25;

/// Probability the substrate cannot resolve the ambiguity alone, given that every tier
/// actually ran and returned no signal. This is `1.0` **by construction of the ladder** —
/// which is precisely why the ladder must be attempted before asking.
pub const P_UNRESOLVABLE_AFTER_LADDER: f64 = 1.0;

/// Probability the substrate cannot resolve the ambiguity when a tier could not even be
/// attempted (dependency absent, e.g. no world model) or the lifetime budget ran out.
/// The search was incomplete, so the substrate must not claim it exhausted its own means.
pub const P_UNRESOLVABLE_INCOMPLETE: f64 = 0.5;

const ENV_CHECK_WINDOW: usize = 20;
const FAILURE_KEYWORDS: &[&str] = &[
    "failed", "unavailable", "offline", "error", "timeout", "blocked", "unreachable",
];

/// Reflexion action under which each rung of the ladder is persisted.
pub const CLARIFY_TIER_ACTION: &str = "clarify_tier";
/// Reflexion action under which each ladder *invocation* is counted.
pub const CLARIFY_CYCLE_ACTION: &str = "clarify_cycle";
/// Reflexion action under which the *reason* the ladder stopped is persisted.
///
/// Without this, the four exit paths (budget, no-progress, exhaustion, lifetime) would be
/// indistinguishable from the outside: each of them re-enters `ask_or_decline`, finds T3
/// already recorded as `asked`, and returns. The observable outcome — and therefore the
/// ledger — would be identical. Recording the reason is what makes the exit a *decision*
/// rather than a silence, and it is bounded by construction: the reason is keyed by its
/// prefix, and there are exactly four call sites, so at most four of these exist per goal.
pub const CLARIFY_EXIT_ACTION: &str = "clarify_exit";

#[derive(Debug, Clone)]
pub enum ClarifyTrigger {
    /// No AC at all: `acceptance_criteria` empty and `success_factors` empty.
    EmptyAC,
    /// AC present but not decidable by a machine: no factor carries an
    /// `observation_pattern`. Distinct from `EmptyAC` — the absent case is not the
    /// unmeasurable case, and only the latter has prior art worth searching for.
    UntestableAC,
    RepeatedVeto { veto_count: u32 },
    PreSuccess,
}

impl ClarifyTrigger {
    /// True for triggers whose subject *is* the acceptance criteria.
    ///
    /// Only these are short-circuited by a fully decidable AC. `PreSuccess` and
    /// `RepeatedVeto` are raised when the AC is already satisfied or when a critic keeps
    /// vetoing, so a decidable AC does not make them moot — short-circuiting them would
    /// silently disable the pre-success and veto clarification paths.
    pub fn is_ac_trigger(&self) -> bool {
        matches!(self, Self::EmptyAC | Self::UntestableAC)
    }
}

/// Which rung resolved the goal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClarifySource {
    Environment,
    PriorArt,
    Causal,
}

impl ClarifySource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Environment => "environment",
            Self::PriorArt => "prior_art",
            Self::Causal => "causal",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClarifyOutcome {
    /// A rung resolved it. `evidence` holds the record ids or the equation name that did.
    ClarifiedBySubstrate {
        source: ClarifySource,
        evidence: Vec<String>,
    },
    /// No clarification needed — either the AC was already decidable, or the ask-cost gate
    /// declined to spend a human's attention on this goal.
    AlreadyClear,
    NeedsUserClarification,
}

/// The rungs of the ladder, in the order they were attempted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClarifyTier {
    T0Environment,
    T1PriorArt,
    T2Causal,
    T3AskUser,
}

impl ClarifyTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::T0Environment => "T0_environment",
            Self::T1PriorArt => "T1_prior_art",
            Self::T2Causal => "T2_causal",
            Self::T3AskUser => "T3_ask_user",
        }
    }

    /// Internal cost of *attempting* the rung, in substrate units. Debited whether or not
    /// the rung resolves — the search itself is the expense, and T3's cost is a user's
    /// attention, which is not debited from the substrate budget.
    pub fn cost(&self) -> f64 {
        match self {
            Self::T0Environment => SubstrateBudget::C_MIN,
            Self::T1PriorArt => 2.0,
            Self::T2Causal => 4.0,
            Self::T3AskUser => 0.0,
        }
    }

    /// The three search rungs, in descent order.
    pub fn search_rungs() -> [ClarifyTier; MAX_CLARIFY_TIERS as usize] {
        [Self::T0Environment, Self::T1PriorArt, Self::T2Causal]
    }
}

impl std::fmt::Display for ClarifyTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Per-goal clarify spend ceiling.
///
/// The spend is **derived from the persisted ladder**, not from a mutable counter: the
/// `clarify_tier` Reflexions are the ledger, so the bound is restart-safe and cannot drift
/// from the rungs it is supposed to bound.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SubstrateBudget {
    pub b0: f64,
}

impl SubstrateBudget {
    /// Cost of the cheapest rung. `K_max = floor(b0 / C_MIN)` is the number of rungs the
    /// budget can pay for — the almost-sure-termination argument, made concrete.
    pub const C_MIN: f64 = 1.0;

    /// The budget that agrees with `MAX_CLARIFY_CYCLES_PER_GOAL` instead of fighting it.
    pub fn default_b0() -> f64 {
        MAX_CLARIFY_CYCLES_PER_GOAL as f64 * Self::C_MIN
    }

    /// Rungs this budget can afford.
    pub fn k_max(&self) -> u32 {
        if self.b0 <= 0.0 {
            0
        } else {
            (self.b0 / Self::C_MIN).floor() as u32
        }
    }

    pub fn exhausted(&self, spend: f64) -> bool {
        spend >= self.b0
    }
}

impl Default for SubstrateBudget {
    fn default() -> Self {
        Self { b0: Self::default_b0() }
    }
}

/// One rung as persisted, and as reported by §3.5 (`clarify_ladder`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LadderRung {
    pub tier: String,
    /// `resolved` | `no_signal` | `unavailable` | `asked` | `declined` | `already_decidable`
    pub outcome: String,
    pub evidence: Vec<String>,
}

/// What the T2 rung found.
enum CausalRung {
    /// A broken equation above the confidence gate — the goal is now explained.
    Resolved { node_id: String, confidence: f64 },
    /// The rung ran and found nothing to repair.
    NoSignal { reason: String },
    /// The rung could not run at all (no world model, or attribution errored). Kept
    /// distinct from `NoSignal` so the ask-cost gate can tell "searched and empty" from
    /// "never searched" — they justify different probabilities.
    Unavailable { reason: String },
}

pub struct ClarifyEngine;

impl ClarifyEngine {
    /// Detect environment-blocked success_factors and rewrite them in-store.
    ///
    /// Scans the last ENV_CHECK_WINDOW Temporal records for failure signals that
    /// overlap each unsatisfied success_factor name. If blocked: renames the factor
    /// to `{name}_when_available` and writes Reflexion{goal_restated}.
    /// Returns true if any factor was restated.
    pub fn restate_if_env_changed<B: crate::persistence::MemoryBackend>(
        store: &mut crate::memory_store::MemoryStore<B>,
        goal_id: Uuid,
        actor: &str,
    ) -> bool {
        use crate::memory_record::{MemoryRecord, MemoryType};
        use crate::payloads::GoalPayload;

        let goal_rec = match store.find_by_id(goal_id).cloned() {
            Some(r) => r,
            None => return false,
        };
        let mut goal: GoalPayload = match serde_json::from_value(goal_rec.metadata.clone()) {
            Ok(p) => p,
            Err(_) => return false,
        };

        // Gather recent Temporal content as env signal evidence.
        let recent: Vec<String> = store
            .all_by_type(MemoryType::Temporal)
            .into_iter()
            .rev()
            .take(ENV_CHECK_WINDOW)
            .map(|r| format!("{} {}", r.action, r.target).to_lowercase())
            .collect();

        let mut any_restated = false;
        let mut blocked_factors: Vec<String> = Vec::new();
        for factor in &mut goal.success_factors {
            if factor.satisfied {
                continue;
            }
            // Skip already-restated factors (idempotent).
            if factor.name.ends_with("_when_available") {
                continue;
            }

            let factor_keywords: Vec<String> = factor
                .name
                .split(|c: char| !c.is_alphanumeric())
                .filter(|s| s.len() > 2)
                .map(|s| s.to_lowercase())
                .collect();

            let env_blocked = recent.iter().any(|content| {
                let has_fail = FAILURE_KEYWORDS.iter().any(|kw| content.contains(kw));
                let has_kw = factor_keywords.iter().any(|kw| content.contains(kw.as_str()));
                has_fail && has_kw
            });

            if env_blocked {
                blocked_factors.push(factor.name.clone());
                factor.name = format!("{}_when_available", factor.name);
                any_restated = true;
            }
        }

        if any_restated {
            if let Ok(meta) = serde_json::to_value(&goal) {
                let _ = store.update_record(goal_id, None, None, None, None, Some(meta));
            }
            let note = format!("environment-blocked success_factors restated for goal {}", goal_id);
            let mut rec = MemoryRecord::new(
                MemoryType::Reflexion,
                actor.to_string(),
                "goal_restated".to_string(),
                goal_id.to_string(),
                serde_json::json!({ "note": note }),
            );
            rec.derived_from = Some(goal_id);
            let _ = store.add(rec);

            // Write probe_required Temporal per blocked factor so the next react
            // iteration knows to probe rather than retry the same react step.
            for blocked in &blocked_factors {
                let mut probe_rec = MemoryRecord::new(
                    MemoryType::Temporal,
                    actor.to_string(),
                    "probe_required".to_string(),
                    blocked.clone(),
                    serde_json::json!({ "reason": "env_blocked_ac" }),
                );
                probe_rec.derived_from = Some(goal_id);
                let _ = store.add(probe_rec);
            }
        }

        any_restated
    }

    /// Run the clarify ladder for `goal_id` under the default substrate budget.
    ///
    /// Signature preserved for existing callers (`loop_engine`, `substrate_daemon`).
    pub fn run<B: crate::persistence::MemoryBackend>(
        store: &mut crate::memory_store::MemoryStore<B>,
        goal_id: Uuid,
        actor: &str,
        trigger: ClarifyTrigger,
        wm: Option<&crate::world_model_enhanced::WorldModelEnhanced>,
    ) -> ClarifyOutcome {
        Self::run_with_budget(store, goal_id, actor, trigger, wm, SubstrateBudget::default())
    }

    /// Run the clarify ladder with an explicit budget.
    ///
    /// Descends T0 → T1 → T2, skipping any rung already recorded for this goal, then falls
    /// through to T3. The module header numbers the six exit guarantees; each is enforced
    /// at the marked site below.
    pub fn run_with_budget<B: crate::persistence::MemoryBackend>(
        store: &mut crate::memory_store::MemoryStore<B>,
        goal_id: Uuid,
        actor: &str,
        trigger: ClarifyTrigger,
        wm: Option<&crate::world_model_enhanced::WorldModelEnhanced>,
        budget: SubstrateBudget,
    ) -> ClarifyOutcome {
        use crate::payloads::GoalPayload;

        // --- Exit guarantee 3: lifetime budget, counted from the persisted ledger. ---
        let cycle_no = Self::ladder_cycles(store, goal_id) + 1;
        Self::record_cycle(store, goal_id, actor, cycle_no);
        if cycle_no > MAX_CLARIFY_CYCLES_PER_GOAL {
            return Self::ask_or_decline(
                store,
                goal_id,
                actor,
                &trigger,
                P_UNRESOLVABLE_INCOMPLETE,
                vec![format!(
                    "lifetime_budget_exhausted: cycle {cycle_no} > {MAX_CLARIFY_CYCLES_PER_GOAL}"
                )],
                None,
            );
        }

        // A goal record the substrate cannot even read is a substrate problem, not an
        // ambiguity the user can resolve — so neither of these may be reported as
        // `NeedsUserClarification` without saying so. When they were, the response claimed the
        // substrate had searched and found it needed help while the ledger held no rungs at all:
        // the receipt and the trace disagreed, and spec §3.5 ("so a human can see exactly why
        // the substrate did or did not ask") was violated for precisely the case a human most
        // needs to see. Record the reason, so the trace names it.
        let Some(goal_rec) = store.find_by_id(goal_id) else {
            Self::record_exit_reason(
                store,
                goal_id,
                actor,
                &[format!("goal_missing: no Goal record for {goal_id}")],
            );
            return ClarifyOutcome::NeedsUserClarification;
        };
        let goal: GoalPayload = match serde_json::from_value(goal_rec.metadata.clone()) {
            Ok(p) => p,
            Err(e) => {
                Self::record_exit_reason(
                    store,
                    goal_id,
                    actor,
                    &[format!(
                        "goal_metadata_unparseable: {e} \
                         (a Goal record must carry at least `target_state`; the ladder \
                         cannot reason about a problem statement it cannot read)"
                    )],
                );
                return ClarifyOutcome::NeedsUserClarification;
            }
        };

        // "A goal with testable AC never reaches T3": if every factor already carries a
        // machine-checkable pattern, the problem statement is not ambiguous. Scoped to AC
        // triggers — see `ClarifyTrigger::is_ac_trigger` for why the other two are exempt.
        if trigger.is_ac_trigger()
            && !goal.success_factors.is_empty()
            && goal.success_factors.iter().all(|f| f.observation_pattern.is_some())
        {
            Self::record_rung(
                store,
                goal_id,
                actor,
                ClarifyTier::T0Environment,
                "already_decidable",
                vec!["every success_factor carries an observation_pattern".into()],
            );
            return ClarifyOutcome::AlreadyClear;
        }

        // --- Exit guarantee 5: substrate budget, derived from the ledger. ---
        let spend: f64 = Self::ladder_rungs(store, goal_id)
            .iter()
            .map(|r| Self::tier_cost(&r.tier))
            .sum();
        if budget.exhausted(spend) {
            return Self::ask_or_decline(
                store,
                goal_id,
                actor,
                &trigger,
                P_UNRESOLVABLE_INCOMPLETE,
                vec![format!(
                    "substrate_budget_exhausted: spend={spend} >= b0={}",
                    budget.b0
                )],
                None,
            );
        }

        // --- Exit guarantees 1 + 2: monotone descent; a consumed rung is never re-entered. ---
        let mut rungs_closed = 0u32;
        for tier in ClarifyTier::search_rungs() {
            if Self::tier_rung(store, goal_id, tier).is_some() {
                continue;
            }
            rungs_closed += 1;
            match tier {
                ClarifyTier::T0Environment => {
                    if Self::restate_if_env_changed(store, goal_id, actor) {
                        let evidence = Self::restated_factor_evidence(store, goal_id);
                        Self::record_rung(store, goal_id, actor, tier, "resolved", evidence.clone());
                        return ClarifyOutcome::ClarifiedBySubstrate {
                            source: ClarifySource::Environment,
                            evidence,
                        };
                    }
                    Self::record_rung(
                        store,
                        goal_id,
                        actor,
                        tier,
                        "no_signal",
                        vec!["no failure-keyword × factor-keyword overlap in recent Temporal records".into()],
                    );
                    // Preserved legacy contract, deliberately evaluated *after* T0: the
                    // caller declared an empty AC but the goal already carries factors, and
                    // the environment offers no reason to restate them — so there is nothing
                    // to clarify. It must not run earlier, or an environment-blocked goal
                    // (which also arrives with factors + `EmptyAC`) would never get to
                    // restate and would be silently declared clear.
                    if !goal.success_factors.is_empty() && matches!(trigger, ClarifyTrigger::EmptyAC) {
                        return ClarifyOutcome::AlreadyClear;
                    }
                }
                ClarifyTier::T1PriorArt => {
                    match Self::prior_art_restatement(store, goal_id, actor, &goal) {
                        Some(evidence) => {
                            Self::record_rung(store, goal_id, actor, tier, "resolved", evidence.clone());
                            return ClarifyOutcome::ClarifiedBySubstrate {
                                source: ClarifySource::PriorArt,
                                evidence,
                            };
                        }
                        None => Self::record_rung(
                            store,
                            goal_id,
                            actor,
                            tier,
                            "no_signal",
                            vec![format!(
                                "no Succeeded goal, Skill or Belief with overlap >= {PRIOR_ART_OVERLAP_GATE}"
                            )],
                        ),
                    }
                }
                ClarifyTier::T2Causal => match Self::causal_attribution(store, goal_id, wm, &goal) {
                    CausalRung::Resolved { node_id, confidence } => {
                        let evidence = vec![
                            format!("broken_equation={node_id}"),
                            format!("confidence={confidence:.3}"),
                            format!("gate={CAUSAL_CONFIDENCE_GATE}"),
                        ];
                        Self::record_causal_repair(store, goal_id, actor, &node_id, confidence);
                        Self::record_rung(store, goal_id, actor, tier, "resolved", evidence.clone());
                        return ClarifyOutcome::ClarifiedBySubstrate {
                            source: ClarifySource::Causal,
                            evidence,
                        };
                    }
                    CausalRung::NoSignal { reason } => {
                        Self::record_rung(store, goal_id, actor, tier, "no_signal", vec![reason]);
                    }
                    CausalRung::Unavailable { reason } => {
                        // Descend anyway so T3 is reached with an accurate probability:
                        // an un-attempted rung means the search was incomplete, not empty.
                        Self::record_rung(store, goal_id, actor, tier, "unavailable", vec![reason]);
                    }
                },
                ClarifyTier::T3AskUser => unreachable!("T3 is terminal and is not a search rung"),
            }
        }

        // --- Exit guarantee 4: no-progress exit. ---
        // Every rung was already consumed, so a further descent would rewrite rungs that
        // already exist. That is repetition, and repetition is what the ladder replaced.
        if rungs_closed == 0 {
            let incomplete = Self::has_unavailable_rung(store, goal_id);
            return Self::ask_or_decline(
                store,
                goal_id,
                actor,
                &trigger,
                Self::p_unresolvable(incomplete),
                vec![format!(
                    "no_progress: all rungs already consumed ({} recorded)",
                    Self::ladder_rungs(store, goal_id).len()
                )],
                None,
            );
        }

        // Ladder exhausted. `P` is 1.0 only if every rung actually ran: if one was
        // unavailable, the substrate never had the means to search and must not claim it did.
        let incomplete = Self::has_unavailable_rung(store, goal_id);
        Self::ask_or_decline(
            store,
            goal_id,
            actor,
            &trigger,
            Self::p_unresolvable(incomplete),
            vec![format!("ladder_exhausted: {MAX_CLARIFY_TIERS} rungs attempted")],
            Some(&goal),
        )
    }

    /// T3 — the terminal rung: ask the user exactly once, or decline to spend their attention.
    ///
    /// Exit guarantee 6. Asks iff
    /// `P(unresolvable) * cost_of_wrong_execution > cost_of_asking`, where
    /// `cost_of_wrong_execution` is the goal's own declared `estimated_cost` — the only risk
    /// signal a goal carries, so no new field and no second truth about risk.
    ///
    /// The decision is persisted (`clarify_ask_declined`) either way: a substrate that
    /// silently declined to ask would be indistinguishable from one that failed to.
    fn ask_or_decline<B: crate::persistence::MemoryBackend>(
        store: &mut crate::memory_store::MemoryStore<B>,
        goal_id: Uuid,
        actor: &str,
        trigger: &ClarifyTrigger,
        p_unresolvable: f64,
        mut evidence: Vec<String>,
        goal: Option<&crate::payloads::GoalPayload>,
    ) -> ClarifyOutcome {
        use crate::memory_record::{MemoryRecord, MemoryType};
        use crate::payloads::{BeliefPayload, GoalPayload};

        // Terminal: once asked, never ask again, and never append a second T3 rung.
        if matches!(
            Self::tier_rung(store, goal_id, ClarifyTier::T3AskUser),
            Some(r) if r.outcome == "asked"
        ) {
            // The rung is already on the ledger, so the *outcome* here carries no new
            // information — but the *reason for being here again* does. Record it, so the
            // no-progress exit is distinguishable from budget exhaustion in the trace.
            Self::record_exit_reason(store, goal_id, actor, &evidence);
            return ClarifyOutcome::NeedsUserClarification;
        }

        let fallback;
        let goal = match goal {
            Some(g) => g,
            None => {
                fallback = store
                    .find_by_id(goal_id)
                    .and_then(|r| serde_json::from_value::<GoalPayload>(r.metadata.clone()).ok())
                    .unwrap_or_default();
                &fallback
            }
        };

        let cost_of_wrong_execution = if goal.estimated_cost > 0.0 {
            goal.estimated_cost
        } else {
            // `GoalPayload` derives `Default`, so an in-memory goal carries `0.0` here —
            // the serde default (`1.0`) only applies when the payload is *parsed*, not when
            // it is `Default::default()`-constructed. Treat a non-positive cost as "risk not
            // declared" and fall back to the declared default, otherwise the gate would read
            // an absent number as "executing this wrongly is free" and never ask.
            crate::payloads::DEFAULT_GOAL_COST
        };
        let ask = p_unresolvable * cost_of_wrong_execution > COST_OF_ASKING;
        evidence.push(format!("p_unresolvable={p_unresolvable}"));
        evidence.push(format!("cost_of_wrong_execution={cost_of_wrong_execution}"));
        evidence.push(format!("cost_of_asking={COST_OF_ASKING}"));

        if !ask {
            Self::record_rung(store, goal_id, actor, ClarifyTier::T3AskUser, "declined", evidence);
            let mut rec = MemoryRecord::new(
                MemoryType::Reflexion,
                actor.to_string(),
                "clarify_ask_declined".to_string(),
                goal_id.to_string(),
                serde_json::json!({
                    "reason": "ask_cost_gate",
                    "p_unresolvable": p_unresolvable,
                    "estimated_cost": cost_of_wrong_execution,
                    "threshold": COST_OF_ASKING,
                }),
            );
            rec.derived_from = Some(goal_id);
            let _ = store.add(rec);
            // Keep the current reading; `max_react_iterations` remains the outer bound.
            return ClarifyOutcome::AlreadyClear;
        }

        // Exactly one `clarify_needed` Belief per goal, ever.
        let already_flagged = store.all_by_type(MemoryType::Belief).into_iter().any(|r| {
            r.actor == actor && r.action == "clarify_needed" && r.derived_from == Some(goal_id)
        });
        if !already_flagged {
            // The *questions* are authored by `agent_guidance` — stateless text generation.
            // Who asks, when, and whether asking is worth it stays here: one owner (C2).
            let authored = crate::agent_guidance::clarify_goal(&goal.target_state);
            let bp = BeliefPayload {
                proposition: format!(
                    "goal {} requires user clarification (trigger={:?}, p_unresolvable={})",
                    goal_id, trigger, p_unresolvable
                ),
                confidence: 0.1,
                clarifying_questions: authored.clarifying_questions,
                ..Default::default()
            };
            if let Ok(meta) = serde_json::to_value(&bp) {
                let mut rec = MemoryRecord::new(
                    MemoryType::Belief,
                    actor.to_string(),
                    "clarify_needed".to_string(),
                    goal_id.to_string(),
                    meta,
                );
                rec.derived_from = Some(goal_id);
                let _ = store.add(rec);
            }
        }
        Self::record_rung(store, goal_id, actor, ClarifyTier::T3AskUser, "asked", evidence);
        ClarifyOutcome::NeedsUserClarification
    }

    /// Probability the substrate cannot resolve the ambiguity alone.
    fn p_unresolvable(incomplete_search: bool) -> f64 {
        if incomplete_search {
            P_UNRESOLVABLE_INCOMPLETE
        } else {
            P_UNRESOLVABLE_AFTER_LADDER
        }
    }

    /// True when any rung could not be attempted at all (as opposed to attempted and empty).
    fn has_unavailable_rung<B: crate::persistence::MemoryBackend>(
        store: &crate::memory_store::MemoryStore<B>,
        goal_id: Uuid,
    ) -> bool {
        Self::ladder_rungs(store, goal_id)
            .iter()
            .any(|r| r.outcome == "unavailable")
    }

    // ── Ladder ledger ────────────────────────────────────────────────────────────────
    //
    // The ledger *is* the `clarify_tier` Reflexions, read back from the store. Keeping the
    // bound in the same place as the rungs it bounds means monotonicity, the spend and the
    // report cannot disagree with each other, and all three survive a restart.

    /// Every rung recorded for `goal_id`, in the order recorded.
    pub fn ladder_rungs<B: crate::persistence::MemoryBackend>(
        store: &crate::memory_store::MemoryStore<B>,
        goal_id: Uuid,
    ) -> Vec<LadderRung> {
        use crate::memory_record::MemoryType;
        store
            .all_by_type(MemoryType::Reflexion)
            .into_iter()
            .filter(|r| r.action == CLARIFY_TIER_ACTION && r.derived_from == Some(goal_id))
            .filter_map(|r| serde_json::from_value::<LadderRung>(r.metadata.clone()).ok())
            .collect()
    }

    /// The rung recorded for `tier`, if the tier has already fired for this goal.
    fn tier_rung<B: crate::persistence::MemoryBackend>(
        store: &crate::memory_store::MemoryStore<B>,
        goal_id: Uuid,
        tier: ClarifyTier,
    ) -> Option<LadderRung> {
        let want = tier.to_string();
        Self::ladder_rungs(store, goal_id)
            .into_iter()
            .find(|r| r.tier == want)
    }

    fn record_rung<B: crate::persistence::MemoryBackend>(
        store: &mut crate::memory_store::MemoryStore<B>,
        goal_id: Uuid,
        actor: &str,
        tier: ClarifyTier,
        outcome: &str,
        evidence: Vec<String>,
    ) {
        use crate::memory_record::{MemoryRecord, MemoryType};
        let rung = LadderRung {
            tier: tier.to_string(),
            outcome: outcome.to_string(),
            evidence,
        };
        let mut rec = MemoryRecord::new(
            MemoryType::Reflexion,
            actor.to_string(),
            CLARIFY_TIER_ACTION.to_string(),
            tier.to_string(),
            serde_json::to_value(&rung).unwrap_or_else(|_| serde_json::json!({})),
        );
        rec.derived_from = Some(goal_id);
        let _ = store.add(rec);
    }

    fn record_cycle<B: crate::persistence::MemoryBackend>(
        store: &mut crate::memory_store::MemoryStore<B>,
        goal_id: Uuid,
        actor: &str,
        cycle_no: u32,
    ) {
        use crate::memory_record::{MemoryRecord, MemoryType};
        let mut rec = MemoryRecord::new(
            MemoryType::Reflexion,
            actor.to_string(),
            CLARIFY_CYCLE_ACTION.to_string(),
            goal_id.to_string(),
            serde_json::json!({ "cycle": cycle_no }),
        );
        rec.derived_from = Some(goal_id);
        let _ = store.add(rec);
    }

    /// The distinct reasons the ladder has stopped for `goal_id`.
    ///
    /// Bounded by construction: at most one entry per exit path (there are four), because
    /// `record_exit_reason` dedups on the reason itself.
    pub fn ladder_exit_reasons<B: crate::persistence::MemoryBackend>(
        store: &crate::memory_store::MemoryStore<B>,
        goal_id: Uuid,
    ) -> Vec<String> {
        use crate::memory_record::MemoryType;
        store
            .all_by_type(MemoryType::Reflexion)
            .into_iter()
            .filter(|r| r.action == CLARIFY_EXIT_ACTION && r.derived_from == Some(goal_id))
            .map(|r| r.target.clone())
            .collect()
    }

    /// Persist *why* the ladder stopped, deduped by reason.
    ///
    /// Called only on re-entry: the first exit already wrote its reason as the T3 rung's
    /// evidence, so this exists to stop a later, different exit from being silently
    /// swallowed by the T3 dedup.
    fn record_exit_reason<B: crate::persistence::MemoryBackend>(
        store: &mut crate::memory_store::MemoryStore<B>,
        goal_id: Uuid,
        actor: &str,
        evidence: &[String],
    ) {
        use crate::memory_record::{MemoryRecord, MemoryType};
        let reason = evidence
            .first()
            .map(|e| e.split(':').next().unwrap_or(e.as_str()).trim().to_string())
            .unwrap_or_else(|| "unspecified".to_string());

        let seen = store
            .all_by_type(MemoryType::Reflexion)
            .into_iter()
            .filter(|r| r.action == CLARIFY_EXIT_ACTION && r.derived_from == Some(goal_id))
            .any(|r| r.target == reason);
        if seen {
            return;
        }

        let mut rec = MemoryRecord::new(
            MemoryType::Reflexion,
            actor.to_string(),
            CLARIFY_EXIT_ACTION.to_string(),
            reason,
            serde_json::json!({ "evidence": evidence }),
        );
        rec.derived_from = Some(goal_id);
        let _ = store.add(rec);
    }

    /// How many times the ladder has been invoked for `goal_id` (this invocation excluded).
    pub fn ladder_cycles<B: crate::persistence::MemoryBackend>(
        store: &crate::memory_store::MemoryStore<B>,
        goal_id: Uuid,
    ) -> u32 {        use crate::memory_record::MemoryType;
        store
            .all_by_type(MemoryType::Reflexion)
            .into_iter()
            .filter(|r| r.action == CLARIFY_CYCLE_ACTION && r.derived_from == Some(goal_id))
            .count() as u32
    }

    fn tier_cost(tier: &str) -> f64 {
        ClarifyTier::search_rungs()
            .iter()
            .find(|t| t.as_str() == tier)
            .map(|t| t.cost())
            .unwrap_or(0.0)
    }

    /// Names of factors a T0 restatement renamed (the persisted evidence for the rung).
    fn restated_factor_evidence<B: crate::persistence::MemoryBackend>(
        store: &crate::memory_store::MemoryStore<B>,
        goal_id: Uuid,
    ) -> Vec<String> {
        use crate::payloads::GoalPayload;
        store
            .find_by_id(goal_id)
            .and_then(|r| serde_json::from_value::<GoalPayload>(r.metadata.clone()).ok())
            .map(|g| {
                g.success_factors
                    .iter()
                    .filter(|f| f.name.ends_with("_when_available"))
                    .map(|f| format!("restated_factor={}", f.name))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Record the repair target a T2 rung identified.
    ///
    /// The rung *decides* and *names the equation*; applying the rewrite belongs to whoever
    /// holds a `CognitiveState` (the daemon does, at `substrate_daemon.rs`). Applying it
    /// from inside the ladder is not possible without a second lock on the same store, and
    /// re-deriving coefficients here would be a second truth about structural drift — which
    /// `prediction_monitor` already owns.
    fn record_causal_repair<B: crate::persistence::MemoryBackend>(
        store: &mut crate::memory_store::MemoryStore<B>,
        goal_id: Uuid,
        actor: &str,
        node_id: &str,
        confidence: f64,
    ) {
        use crate::memory_record::{MemoryRecord, MemoryType};
        let mut rec = MemoryRecord::new(
            MemoryType::Reflexion,
            actor.to_string(),
            "clarify_causal_repair".to_string(),
            node_id.to_string(),
            serde_json::json!({
                "node_id": node_id,
                "confidence": confidence,
                "proposed_delta": "RewriteStructuralEquation",
            }),
        );
        rec.derived_from = Some(goal_id);
        let _ = store.add(rec);
    }

    // ── T1: prior-art restatement ────────────────────────────────────────────────────

    /// Search the substrate's own past for an AC this goal can adopt.
    ///
    /// Three sources, in descending order of specificity:
    ///   1. a **Succeeded goal** whose success factors overlap ≥ `PRIOR_ART_OVERLAP_GATE`
    ///      with this goal — its AC and its *patterned* factors are adopted verbatim;
    ///   2. a **Skill** whose `expected_outcomes` name this goal's target — its outcomes
    ///      become factors, made decidable by `agent_guidance::plan_validation`;
    ///   3. a **Belief** whose proposition mentions this goal's target — same treatment.
    ///
    /// Why beliefs are still searched: the pre-ladder engine resolved on exactly that
    /// evidence. Dropping it to match the spec's `Reflexion + Skill` wording would delete a
    /// working self-resolution path, and the spec's intent is to *add* prior-art sources.
    /// What changes is that a hit must now produce a clause that can be *machine-checked* —
    /// previously it returned `ClarifiedBySubstrate` having written nothing, so the caller
    /// reloaded a goal with no factors and immediately returned `Pending`.
    ///
    /// Returns the evidence ids when it resolved, `None` when there is no prior art.
    fn prior_art_restatement<B: crate::persistence::MemoryBackend>(
        store: &mut crate::memory_store::MemoryStore<B>,
        goal_id: Uuid,
        actor: &str,
        goal: &crate::payloads::GoalPayload,
    ) -> Option<Vec<String>> {
        use crate::memory_record::{MemoryRecord, MemoryType};
        use crate::payloads::{BeliefPayload, GoalPayload, SkillPayload, SuccessFactor};

        let mut adopted: Vec<String> = Vec::new();
        let mut new_factors: Vec<SuccessFactor> = Vec::new();
        let mut evidence: Vec<String> = Vec::new();
        let mut extras: Vec<String> = Vec::new();

        // ── 0. Untestable AC: prose criteria but no factors. ──
        //
        // This is the `UntestableAC` primary resolver (§3.4). "Untestable" here does not mean
        // the factor lacks an `observation_pattern` — in this engine a `None` pattern is
        // perfectly decidable via the two-hit rule in
        // `loop_engine::score_success_factors_from_intents`. It means the goal stated what
        // "done" looks like in prose and never turned it into a single factor the substrate
        // can score. Translating each criterion into a factor *is* the clarification: the
        // criteria already say what is required, so there is nothing a human could add that
        // the substrate does not already have. No question is worth asking for this class,
        // which is why an `UntestableAC` goal resolves here rather than descending to T3.
        if goal.success_factors.is_empty() && !goal.acceptance_criteria.is_empty() {
            for ac in &goal.acceptance_criteria {
                new_factors.push(Self::factor_from_outcome(ac));
            }
            evidence.push(format!(
                "untestable_ac_resolved: criteria_to_factors={}",
                new_factors.len()
            ));
        }

        let mine: std::collections::HashSet<String> = goal
            .success_factors
            .iter()
            .flat_map(|f| Self::tokens(&f.name))
            .collect();

        // ── 1. A Succeeded goal with overlapping factors. ──
        let mut best: Option<(f64, Uuid, GoalPayload)> = None;
        for rec in store.all_by_type(MemoryType::Goal) {
            if rec.id == goal_id {
                continue;
            }
            let Ok(prior) = serde_json::from_value::<GoalPayload>(rec.metadata.clone()) else {
                continue;
            };
            if prior.status != crate::payloads::GoalStatus::Succeeded {
                continue;
            }
            let overlap = Self::factor_overlap(&mine, &prior);
            if overlap >= PRIOR_ART_OVERLAP_GATE && best.as_ref().map(|(b, ..)| overlap > *b).unwrap_or(true) {
                best = Some((overlap, rec.id, prior));
            }
        }

        if let Some((overlap, prior_id, prior)) = best {
            for ac in &prior.acceptance_criteria {
                if !goal.acceptance_criteria.contains(ac) {
                    adopted.push(ac.clone());
                }
            }
            for f in &prior.success_factors {
                let known = goal
                    .success_factors
                    .iter()
                    .chain(new_factors.iter())
                    .any(|k| k.name == f.name);
                if !known && f.observation_pattern.is_some() {
                    new_factors.push(f.clone());
                }
            }
            evidence.push(format!("prior_art_goal={prior_id}"));
            evidence.push(format!("factor_overlap={overlap:.2}"));
        }

        // ── 2. A Skill whose outcomes name this goal. ──
        if evidence.is_empty() {
            let target_tokens = Self::tokens(&goal.target_state);
            for rec in store.all_by_type(MemoryType::Skill) {
                let Ok(skill) = serde_json::from_value::<SkillPayload>(rec.metadata.clone()) else {
                    continue;
                };
                let hit = skill
                    .expected_outcomes
                    .iter()
                    .any(|o| Self::tokens(o).iter().any(|t| target_tokens.contains(t)));
                if hit && !skill.expected_outcomes.is_empty() {
                    for outcome in &skill.expected_outcomes {
                        new_factors.push(Self::factor_from_outcome(outcome));
                    }
                    evidence.push(format!("prior_art_skill={}", rec.id));
                    extras.push(format!("skill={}", skill.procedure));
                    break;
                }
            }
        }

        // ── 3. A Belief whose proposition mentions this goal's target. ──
        if evidence.is_empty() && !goal.target_state.is_empty() {
            for rec in store.all_by_type(MemoryType::Belief) {
                if rec.action == "clarify_needed" {
                    continue;
                }
                let Ok(bp) = serde_json::from_value::<BeliefPayload>(rec.metadata.clone()) else {
                    continue;
                };
                if bp.proposition.contains(goal.target_state.as_str()) {
                    new_factors.push(Self::factor_from_outcome(&bp.proposition));
                    evidence.push(format!("prior_art_belief={}", rec.id));
                    break;
                }
            }
        }

        if evidence.is_empty() {
            return None;
        }

        // Commit: the resolved AC is stored on the goal (as a proposal), and a validation
        // plan is emitted alongside it, per the spec's AC/validation application.
        let mut updated = goal.clone();
        for ac in adopted {
            updated.acceptance_criteria.push(ac);
        }
        for f in new_factors {
            if !updated.success_factors.iter().any(|k| k.name == f.name) {
                updated.success_factors.push(f);
            }
        }
        if let Ok(meta) = serde_json::to_value(&updated) {
            let _ = store.update_record(goal_id, None, None, None, None, Some(meta));
        }

        // Preserved contract + the rung's own breadcrumb.
        let mut self_clarified = MemoryRecord::new(
            MemoryType::Reflexion,
            actor.to_string(),
            "self_clarified".to_string(),
            goal_id.to_string(),
            serde_json::json!({
                "note": format!("resolved_by=prior_art evidence={:?}", evidence),
            }),
        );
        self_clarified.derived_from = Some(goal_id);
        let _ = store.add(self_clarified);

        // Validation plan for the factors we just made decidable.
        let names: Vec<&str> = updated.success_factors.iter().map(|f| f.name.as_str()).collect();
        let plan = crate::agent_guidance::plan_validation(&names);
        let mut plan_rec = MemoryRecord::new(
            MemoryType::Reflexion,
            actor.to_string(),
            "validation_plan".to_string(),
            goal_id.to_string(),
            serde_json::json!({
                "steps": plan.steps,
                "unknown_factors": plan.unknown_factors,
                "validation_order": plan.validation_order,
            }),
        );
        plan_rec.derived_from = Some(goal_id);
        let _ = store.add(plan_rec);

        evidence.extend(extras);
        Some(evidence)
    }

    /// A success factor derived from a prior-art clause.
    ///
    /// `observation_pattern` is deliberately left `None`, and this is a **deviation from the
    /// spec's letter** that is worth stating plainly. The spec says the resolved AC should
    /// gain "a machine-checkable metric/threshold". The obvious source is
    /// `agent_guidance::plan_validation`, which maps a factor to an `expected_evidence`
    /// string — but that string is *prose addressed to an agent* ("test output shows 0
    /// failures"), while `observation_pattern` is read by
    /// `loop_engine::score_success_factors_from_intents` as a literal `content_excerpt`
    /// **substring predicate**. Copying prose into it would not make the factor decidable;
    /// it would make it permanently unsatisfiable, which is worse than the `None` it also
    /// had. A `None` pattern is decidable here — it falls back to the two-hit rule — so the
    /// honest move is to leave the pattern absent and emit the validation plan alongside,
    /// where it is read as instructions by the agent that can act on them.
    fn factor_from_outcome(outcome: &str) -> crate::payloads::SuccessFactor {
        crate::payloads::SuccessFactor {
            name: Self::slug(outcome),
            weight: 1.0,
            satisfied: false,
            observation_pattern: None,
        }
    }

    fn slug(s: &str) -> String {
        let mut out = String::new();
        for c in s.chars() {
            if c.is_alphanumeric() {
                out.extend(c.to_lowercase());
            } else if !out.ends_with('_') && !out.is_empty() {
                out.push('_');
            }
        }
        let trimmed = out.trim_matches('_').to_string();
        if trimmed.is_empty() {
            "clarified_factor".to_string()
        } else if trimmed.len() > 64 {
            trimmed[..64].to_string()
        } else {
            trimmed
        }
    }

    fn tokens(s: &str) -> std::collections::HashSet<String> {
        s.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| t.len() > 2)
            .map(str::to_string)
            .collect()
    }

    /// |mine ∩ theirs| / |mine| — how much of *this* goal's vocabulary the prior goal covers.
    ///
    /// An empty `mine` cannot be covered, so it scores 0.0 and prior art cannot resolve a
    /// goal that never stated a factor: there is nothing to compare a past goal against.
    fn factor_overlap(
        mine: &std::collections::HashSet<String>,
        prior: &crate::payloads::GoalPayload,
    ) -> f64 {
        if mine.is_empty() {
            return 0.0;
        }
        let theirs: std::collections::HashSet<String> = prior
            .success_factors
            .iter()
            .flat_map(|f| Self::tokens(&f.name))
            .collect();
        let hit = mine.intersection(&theirs).count() as f64;
        hit / mine.len() as f64
    }

    // ── T2: causal attribution restatement ───────────────────────────────────────────

    /// Ask the world model which structural equation explains this goal's failure.
    ///
    /// The trajectory is the goal's own `Temporal` steps — the same construction
    /// `loop_engine` uses before declaring `Failed`, so the ladder attributes over the same
    /// evidence rather than inventing a second view of the goal's history.
    ///
    /// **The keys of each step are *causal node ids*, not ReAct field names.** This matters:
    /// `causal::CausalGraph::credit_assign` scores a candidate equation only when
    /// `step.get(node_id)` binds, and it needs `step.get(parent_id)` for every parent too.
    /// An earlier construction keyed each step by `iteration`/`unsatisfied`, which no causal
    /// graph ever declares as a node — so every candidate was skipped, `broken_equation` was
    /// always `None`, and the T2 rung could never fire no matter how badly the goal had gone.
    /// Keying by node id is what makes the rung observable at all.
    ///
    /// The value for a node at a step is how many of the goal's observations in that step
    /// mention the node — the only per-node signal the goal's own history carries. A node
    /// never mentioned still gets a `0.0` entry so that its equation is *scored* (a large
    /// residual is exactly the evidence that its structural equation is the broken one),
    /// rather than silently skipped.
    fn causal_attribution<B: crate::persistence::MemoryBackend>(
        store: &crate::memory_store::MemoryStore<B>,
        goal_id: Uuid,
        wm: Option<&crate::world_model_enhanced::WorldModelEnhanced>,
        _goal: &crate::payloads::GoalPayload,
    ) -> CausalRung {
        use crate::memory_record::MemoryType;
        use crate::world_model_enhanced::causal::FailureSignal;

        let Some(wm) = wm else {
            return CausalRung::Unavailable {
                reason: "no world model supplied to the ladder (wm=None)".into(),
            };
        };

        let node_ids = wm.causal_node_ids();
        if node_ids.is_empty() {
            return CausalRung::NoSignal {
                reason: "causal graph declares no nodes, so no equation can be attributed".into(),
            };
        }

        let steps: Vec<_> = store
            .all()
            .iter()
            .filter(|r| r.record_type == MemoryType::Temporal && r.derived_from == Some(goal_id))
            .cloned()
            .collect();

        if steps.is_empty() {
            return CausalRung::NoSignal {
                reason: "no Temporal steps derived from this goal to attribute over".into(),
            };
        }

        let traj: Vec<std::collections::HashMap<String, f64>> = steps
            .iter()
            .map(|r| {
                let haystack = format!(
                    "{} {} {}",
                    r.target.to_lowercase(),
                    r.action.to_lowercase(),
                    r.metadata.to_string().to_lowercase()
                );
                node_ids
                    .iter()
                    .map(|id| {
                        let needle = id.to_lowercase();
                        let hits = haystack.matches(needle.as_str()).count() as f64;
                        (id.clone(), hits)
                    })
                    .collect()
            })
            .collect();

        match wm.credit_assign_trajectory(&traj, FailureSignal::MaxIterations) {
            Ok(report) => match report.broken_equation {
                Some(node)
                    if report.single_intervention_sufficient
                        && report.confidence >= CAUSAL_CONFIDENCE_GATE =>
                {
                    CausalRung::Resolved {
                        node_id: node,
                        confidence: report.confidence,
                    }
                }
                Some(node) => CausalRung::NoSignal {
                    reason: format!(
                        "broken_equation={node} below gate: confidence={:.3} < {CAUSAL_CONFIDENCE_GATE}",
                        report.confidence
                    ),
                },
                None => CausalRung::NoSignal {
                    reason: "no equation-bearing candidate in the causal graph".into(),
                },
            },
            Err(e) => CausalRung::Unavailable {
                reason: format!("credit_assign_trajectory failed: {e}"),
            },
        }
    }

    /// Apply a proposed GoalRevision by synthesising new success_factors from recently active
    /// intent entities. Called after loop_engine writes Reflexion{goal_revision_proposed=true}.
    ///
    /// Bounded: runs once per GoalRevision emit (consecutive_low_score already reset).
    /// Self-prompts by scanning recent Intent records for active target_entity values, then
    /// adds any entity not already covered as a new SuccessFactor. On failure, writes a
    /// single deduped Belief{clarify_needed, source="goal_revision_drift"} → NeedsUserClarification.
    pub fn apply_revision<B: crate::persistence::MemoryBackend>(
        store: &mut crate::memory_store::MemoryStore<B>,
        goal_id: Uuid,
        actor: &str,
    ) -> ClarifyOutcome {
        use crate::memory_record::{MemoryRecord, MemoryType};
        use crate::payloads::{BeliefPayload, GoalPayload, SuccessFactor};

        let goal_rec = match store.find_by_id(goal_id).cloned() {
            Some(r) => r,
            None => return ClarifyOutcome::NeedsUserClarification,
        };
        let mut goal: GoalPayload = match serde_json::from_value(goal_rec.metadata.clone()) {
            Ok(p) => p,
            Err(_) => return ClarifyOutcome::NeedsUserClarification,
        };

        // Collect recently active intent entities for this actor.
        let active_entities: Vec<String> = store
            .all_by_type(MemoryType::Intent)
            .into_iter()
            .filter(|r| r.actor == actor)
            .filter_map(|r| {
                r.metadata.get("target_entity").and_then(|v| v.as_str()).map(str::to_string)
            })
            .fold(Vec::<String>::new(), |mut acc, e| {
                if !acc.contains(&e) { acc.push(e); }
                acc
            });

        if active_entities.is_empty() {
            // No active entities — escalate to user.
            let bp = BeliefPayload {
                proposition: format!(
                    "goal {} GoalRevision could not synthesise new factors: no active intent entities found",
                    goal_id
                ),
                confidence: 0.1,
                ..Default::default()
            };
            if let Ok(meta) = serde_json::to_value(&bp) {
                // Dedup: only write if no existing clarify_needed for this goal.
                let already = store.all_by_type(MemoryType::Belief).into_iter().any(|r| {
                    r.derived_from == Some(goal_id)
                        && r.action == "clarify_needed"
                        && r.metadata.get("source").and_then(|v| v.as_str()) == Some("goal_revision_drift")
                });
                if !already {
                    let mut rec = MemoryRecord::new(
                        MemoryType::Belief,
                        actor.to_string(),
                        "clarify_needed".to_string(),
                        goal_id.to_string(),
                        {
                            let mut m = meta;
                            m["source"] = serde_json::json!("goal_revision_drift");
                            m
                        },
                    );
                    rec.derived_from = Some(goal_id);
                    let _ = store.add(rec);
                }
            }
            return ClarifyOutcome::NeedsUserClarification;
        }

        // Synthesise new factors for entities not already present.
        let existing_keys: Vec<String> = goal.success_factors.iter()
            .map(|f| f.name.split('_').next().unwrap_or(&f.name).to_string())
            .collect();
        let mut added = 0u32;
        for entity in &active_entities {
            let key = entity.split('_').next().unwrap_or(entity);
            if existing_keys.iter().any(|k| k == key || k.contains(key) || key.contains(k.as_str())) {
                continue;
            }
            goal.success_factors.push(SuccessFactor {
                name: format!("{}_revised", entity),
                weight: 1.0,
                satisfied: false,
                observation_pattern: None,
            });
            added += 1;
        }

        if let Ok(meta) = serde_json::to_value(&goal) {
            let _ = store.update_record(goal_id, None, None, None, None, Some(meta));
        }
        let mut rev = MemoryRecord::new(
            MemoryType::Reflexion,
            actor.to_string(),
            "goal_restated_from_revision".to_string(),
            goal_id.to_string(),
            serde_json::json!({
                "factors_added": added,
                "entities_found": active_entities,
                "source": "apply_revision",
            }),
        );
        rev.derived_from = Some(goal_id);
        let _ = store.add(rev);
        ClarifyOutcome::ClarifiedBySubstrate {
            source: ClarifySource::PriorArt,
            evidence: vec![format!("goal_revision: factors_added={added}")],
        }
    }
}
