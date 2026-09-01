<script lang="ts">
	import { str } from '../util.js';

	let { direction, props }: { direction: 'outgoing' | 'incoming'; props: Record<string, unknown> } =
		$props();
</script>

<div class="widget sms {direction}">
	{#if props.sender}
		<span class="sms-sender">{str(props.sender)}</span>
	{/if}
	<div class="sms-bubble">{str(props.content)}</div>
</div>

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
</style>
