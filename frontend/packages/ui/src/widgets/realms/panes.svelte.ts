/**
 * What the pane area can show. Cast is permanent; everything else opens and
 * closes as you work.
 */
export type Pane = { kind: 'cast' } | { kind: 'script'; agentKey: string };

export function paneId(pane: Pane): string {
	return pane.kind === 'cast' ? 'cast' : `script:${pane.agentKey}`;
}

const CAST: Pane = { kind: 'cast' };

class PaneStore {
	open = $state<Pane[]>([CAST]);
	activeId = $state('cast');

	get active(): Pane {
		return this.open.find((p) => paneId(p) === this.activeId) ?? CAST;
	}

	/** Focus a pane if it is already open, otherwise add it and focus it. */
	show(pane: Pane) {
		const id = paneId(pane);
		if (!this.open.some((p) => paneId(p) === id)) this.open.push(pane);
		this.activeId = id;
	}

	close(id: string) {
		if (id === 'cast') return;
		const at = this.open.findIndex((p) => paneId(p) === id);
		if (at < 0) return;
		this.open.splice(at, 1);
		// Fall back to the neighbour that took its place, or the one before it.
		if (this.activeId === id) {
			this.activeId = paneId(this.open[at] ?? this.open[at - 1] ?? CAST);
		}
	}

	reset() {
		this.open = [CAST];
		this.activeId = 'cast';
	}

	/**
	 * Drop panes whose agent is gone, or is no longer a script. Deleting an
	 * agent from the cast must not leave its editor open over nothing.
	 */
	prune(liveScriptKeys: string[]) {
		const stale = this.open.filter(
			(p) => p.kind === 'script' && !liveScriptKeys.includes(p.agentKey)
		);
		for (const pane of stale) this.close(paneId(pane));
	}
}

export const panes = new PaneStore();
