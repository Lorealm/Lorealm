<script lang="ts">
	import { settings } from '$lib/stores/settings.svelte.js';
</script>

<section class="section">
	<div class="body">
		<h2>Editor</h2>

		<label class="check">
			<input
				type="checkbox"
				checked={settings.current.weaverSuggestCrossBookIds}
				onchange={(e) => settings.set('weaverSuggestCrossBookIds', e.currentTarget.checked)}
			/>
			<span>
				Suggest cross-book ids
				<span class="hint">Complete entry ids from the other active lorebooks.</span>
			</span>
		</label>

		<label class="check">
			<input
				type="checkbox"
				checked={settings.current.weaverFollowCrossBookIds}
				onchange={(e) => settings.set('weaverFollowCrossBookIds', e.currentTarget.checked)}
			/>
			<span>
				Follow cross-book ids
				<span class="hint">Cmd/ctrl-click an id to open the book that owns it.</span>
			</span>
		</label>

		<label class="check">
			<input
				type="checkbox"
				checked={settings.current.autoExpandLorebookAdvanced}
				onchange={(e) => settings.set('autoExpandLorebookAdvanced', e.currentTarget.checked)}
			/>
			<span>
				Auto-expand advanced controls
				<span class="hint">Automatically expands the advanced details in the Lorebook editor.</span>
			</span>
		</label>

		<label class="check">
			<input
				type="checkbox"
				checked={settings.current.weaverHighlightTrimMarkers}
				onchange={(e) => settings.set('weaverHighlightTrimMarkers', e.currentTarget.checked)}
			/>
			<span>
				Highlight trim markers
				<span class="hint">
					Colour the `-` in {'{#- … -#}'} apart from the delimiter around it.
				</span>
			</span>
		</label>
	</div>
</section>

<style>
	.section {
		display: flex;
		flex: 1 1 auto;
		min-height: 0;
		flex-direction: column;
	}
	.body {
		flex: 1 1 auto;
		min-height: 0;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-lg);
		padding: var(--spacing-lg);
	}
	.body h2 {
		font-size: var(--font-size-h4);
	}
	.check {
		display: flex;
		align-items: flex-start;
		gap: var(--spacing-sm);
		cursor: pointer;
	}
	.check > span {
		display: flex;
		flex-direction: column;
		gap: 2px;
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.hint {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
</style>
