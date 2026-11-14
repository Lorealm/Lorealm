<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { settings } from '#lib/stores/settings.svelte.js';
</script>

<section class="section">
	<div class="body">
		<h2>{t.nav.user.editor}</h2>

		<label class="check">
			<input
				type="checkbox"
				checked={settings.current.weaverSuggestCrossBookIds}
				onchange={(e) => settings.set('weaverSuggestCrossBookIds', e.currentTarget.checked)}
			/>
			<span>
				{t.settings.editor.suggestCrossBook}
				<span class="hint">{t.settings.editor.suggestCrossBookHint}</span>
			</span>
		</label>

		<label class="check">
			<input
				type="checkbox"
				checked={settings.current.weaverFollowCrossBookIds}
				onchange={(e) => settings.set('weaverFollowCrossBookIds', e.currentTarget.checked)}
			/>
			<span>
				{t.settings.editor.followCrossBook}
				<span class="hint">{t.settings.editor.followCrossBookHint}</span>
			</span>
		</label>

		<label class="check">
			<input
				type="checkbox"
				checked={settings.current.autoExpandLorebookAdvanced}
				onchange={(e) => settings.set('autoExpandLorebookAdvanced', e.currentTarget.checked)}
			/>
			<span>
				{t.settings.editor.autoExpand}
				<span class="hint">{t.settings.editor.autoExpandHint}</span>
			</span>
		</label>

		<label class="check">
			<input
				type="checkbox"
				checked={settings.current.weaverHighlightTrimMarkers}
				onchange={(e) => settings.set('weaverHighlightTrimMarkers', e.currentTarget.checked)}
			/>
			<span>
				{t.settings.editor.trimMarkers}
				<span class="hint">{t.settings.editor.trimMarkersHint}</span>
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
