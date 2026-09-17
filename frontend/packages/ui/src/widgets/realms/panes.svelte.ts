/** What the pane area can show. */
export type Pane =
	| { kind: 'overview' }
	| { kind: 'scenarios' }
	| { kind: 'cast' }
	| { kind: 'document' }
	| { kind: 'director' }
	| { kind: 'script'; agentKey: string };

export function paneId(pane: Pane): string {
	return pane.kind === 'script' ? `script:${pane.agentKey}` : pane.kind;
}

const OVERVIEW: Pane = { kind: 'overview' };
const SCENARIOS: Pane = { kind: 'scenarios' };
const CAST: Pane = { kind: 'cast' };
const DOCUMENT: Pane = { kind: 'document' };

const PERMANENT = [OVERVIEW, SCENARIOS, CAST, DOCUMENT];

class PaneStore {
	open = $state<Pane[]>([...PERMANENT]);
	activeId = $state('overview');

	get active(): Pane {
		return this.open.find((p) => paneId(p) === this.activeId) ?? OVERVIEW;
	}

	/** Focus a pane if it is already open, otherwise add it and focus it. */
	show(pane: Pane) {
		const id = paneId(pane);
		if (!this.open.some((p) => paneId(p) === id)) this.open.push(pane);
		this.activeId = id;
	}

	close(id: string) {
		if (PERMANENT.some((p) => paneId(p) === id)) return;
		const at = this.open.findIndex((p) => paneId(p) === id);
		if (at < 0) return;
		this.open.splice(at, 1);
		// Fall back to the neighbour that took its place, or the one before it.
		if (this.activeId === id) {
			this.activeId = paneId(this.open[at] ?? this.open[at - 1] ?? OVERVIEW);
		}
	}

	reset() {
		this.open = [...PERMANENT];
		this.activeId = 'overview';
	}

	/**
	 * Drop panes whose agent is gone, or is no longer a script.
	 */
	prune(liveScriptKeys: string[]) {
		const stale = this.open.filter(
			(p) => p.kind === 'script' && !liveScriptKeys.includes(p.agentKey)
		);
		for (const pane of stale) this.close(paneId(pane));
	}
}

export const panes = new PaneStore();
