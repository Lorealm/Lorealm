<script lang="ts">
	import type { WidgetInstance } from '$lib/types/widgets.js';

	let { widget }: { widget: WidgetInstance } = $props();

	const str = (v: unknown) => (typeof v === 'string' ? v : v == null ? '' : String(v));
</script>

{#if widget.widget_id === 'sms-outgoing'}
	<div class="widget sms outgoing">
		{#if widget.props.sender}
			<span class="sms-sender">{str(widget.props.sender)}</span>
		{/if}
		<div class="sms-bubble">{str(widget.props.content)}</div>
	</div>
{:else if widget.widget_id === 'sms-incoming'}
	<div class="widget sms incoming">
		{#if widget.props.sender}
			<span class="sms-sender">{str(widget.props.sender)}</span>
		{/if}
		<div class="sms-bubble">{str(widget.props.content)}</div>
	</div>
{:else if widget.widget_id === 'note'}
	<div class="widget note">{str(widget.props.content)}</div>
{:else}
	<div class="widget unknown">Unknown widget: {widget.widget_id}</div>
{/if}

<style>
	.widget {
		max-width: 75%;
	}

	.sms {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		margin-bottom: var(--spacing-sm);
	}

	.sms.outgoing {
		margin-left: auto;
		margin-right: var(--spacing-sm); /* must be >= the tail's 6px offset */
		align-items: flex-end;
	}

	.sms.incoming {
		margin-right: auto;
		margin-left: var(--spacing-sm);
		align-items: flex-start;
	}

	.sms-sender {
		color: var(--color-text-muted);
		font-family: var(--font-family-heading);
		font-size: var(--font-size-xs);
		letter-spacing: var(--letter-spacing-uppercase);
		text-transform: uppercase;
		padding: 0 var(--spacing-md);
	}

	.sms-bubble {
		position: relative;
		width: fit-content;
		color: var(--color-text-inverse);
		font-family: var(--font-family-body);
		border-radius: var(--radius-lg);
		padding: var(--spacing-sm) var(--spacing-md);
		line-height: var(--line-height-normal);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		box-shadow: var(--shadow-sm);
		transition: box-shadow var(--transition-fast) var(--easing-default);
		margin-bottom: var(--spacing-sm);
	}

	.sms-bubble::after {
		content: '';
		position: absolute;
		bottom: 0;
		width: 12px;
		height: 14px;
	}

	/* Outgoing Bubble styling (Pink) */
	.outgoing .sms-bubble {
		background: linear-gradient(
			180deg,
			var(--color-role-user-text) 0%,
			var(--color-role-user) 100%
		);
		border-bottom-right-radius: var(--radius-sm);
	}
	.outgoing .sms-bubble::after {
		right: -6px;
		background: var(--color-role-user); /* matches the 100% gradient stop */
		clip-path: polygon(0 0, 0 100%, 100% 100%);
	}

	/* Incoming Bubble styling (Purple) */
	.incoming .sms-bubble {
		background: linear-gradient(
			180deg,
			var(--color-role-assistant-text) 0%,
			var(--color-role-assistant) 100%
		);
		border-bottom-left-radius: var(--radius-sm);
	}
	.incoming .sms-bubble::after {
		left: -6px;
		background: var(--color-role-assistant);
		clip-path: polygon(100% 0, 100% 100%, 0 100%);
	}

	.sms-bubble:hover {
		box-shadow: var(--shadow-md);
	}

	.note {
		position: relative;
		background-color: var(--color-bg-elevated);
		background-image: var(--gradient-subtle);
		border: 1px solid var(--color-border-medium);
		border-radius: var(--radius-sm);
		padding: var(--spacing-lg) var(--spacing-md);
		font-family: var(--font-family-body-alt);
		font-style: italic;
		line-height: var(--line-height-normal);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		transform: rotate(-0.5deg);
		box-shadow: var(--shadow-md);
		transition:
			transform var(--transition-normal) var(--easing-default),
			box-shadow var(--transition-normal) var(--easing-default);
	}

	.note::before {
		content: '';
		position: absolute;
		top: -9px;
		left: 50%;
		width: 60px;
		height: 18px;
		transform: translateX(-50%) rotate(1.5deg);
		background: var(--color-primary-light);
		opacity: 0.3;
		border-radius: var(--radius-sm);
	}

	.note:hover {
		transform: rotate(0deg);
		box-shadow: var(--shadow-lg);
	}

	.unknown {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
		border: 1px dashed var(--color-border-subtle);
		border-radius: var(--radius-md);
		padding: var(--spacing-sm);
		width: fit-content;
	}
</style>
