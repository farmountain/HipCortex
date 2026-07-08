/**
 * Session token savings tracker for HipCortex.
 *
 * Tracks tokens used by HipCortex context bundles (treatment) vs estimated
 * full-history tokens (baseline). Resets when VS Code is restarted.
 *
 * Token estimation: len(text) / 4 — consistent with /memory/context endpoint.
 * Copilot credit rate: $0.01 per 1000 tokens (approximate for Business plan).
 */

export interface TokenSavingsSnapshot {
    hipCortexTokens: number;
    estimatedBaselineTokens: number;
    savedTokens: number;
    savingsPct: number;
    estimatedCreditsSaved: number;
    sessionCallCount: number;
}

export class TokenTracker {
    private hipCortexTokens = 0;
    private estimatedBaselineTokens = 0;
    private sessionCallCount = 0;

    /** Rough token estimate: len(text) / 4 */
    static estimateTokens(text: string): number {
        return Math.max(1, Math.floor(text.length / 4));
    }

    /**
     * Record a HipCortex memory retrieval event.
     * Supports live_beliefs bundles (summary + world_state) for richer status.
     * @param contextBundle  Text of memory records returned (treatment)
     * @param baselineTokens Estimated tokens if full history had been injected
     */
    record(contextBundle: string, baselineTokens: number): void {
        const usedTokens = TokenTracker.estimateTokens(contextBundle);
        this.hipCortexTokens         += usedTokens;
        this.estimatedBaselineTokens += Math.max(usedTokens, baselineTokens);
        this.sessionCallCount        += 1;
    }

    getSnapshot(): TokenSavingsSnapshot {
        const saved   = Math.max(0, this.estimatedBaselineTokens - this.hipCortexTokens);
        const pct     = this.estimatedBaselineTokens > 0
            ? (saved / this.estimatedBaselineTokens) * 100
            : 0;
        const credits = saved / 1000 * 0.01;
        return {
            hipCortexTokens:         this.hipCortexTokens,
            estimatedBaselineTokens: this.estimatedBaselineTokens,
            savedTokens:             saved,
            savingsPct:              pct,
            estimatedCreditsSaved:   credits,
            sessionCallCount:        this.sessionCallCount,
        };
    }

    /** Formatted footer for @hipcortex chat responses */
    formatSavingsFooter(contextBundleText: string, baselineTokens: number): string {
        const used  = TokenTracker.estimateTokens(contextBundleText);
        const saved = Math.max(0, baselineTokens - used);
        const pct   = baselineTokens > 0 ? Math.round(saved / baselineTokens * 100) : 0;
        const snap  = this.getSnapshot();
        return [
            `---`,
            `*HipCortex used ~${used} tokens (vs ~${baselineTokens} full history = **${pct}% savings**)*`,
            `*Session: ~${snap.savedTokens.toLocaleString()} tokens saved · ~$${snap.estimatedCreditsSaved.toFixed(3)} Copilot credits*`,
        ].join('\n');
    }

    /** Short label for status bar. Updated for richer WM + loops + savings visibility after live_beliefs/reflect. */
    formatStatusBarLabel(wm: number = 0, loops: number = 0): string {
        const snap = this.getSnapshot();
        const saved = snap.savedTokens.toLocaleString();
        if (snap.sessionCallCount === 0 && wm === 0) {
            return '$(database) HipCortex';
        }
        return `$(database) HipCortex: WM ${wm} | loops ${loops} | ${saved} tok`;
    }

    reset(): void {
        this.hipCortexTokens         = 0;
        this.estimatedBaselineTokens = 0;
        this.sessionCallCount        = 0;
    }
}
