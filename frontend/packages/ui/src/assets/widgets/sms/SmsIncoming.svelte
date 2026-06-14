<script lang="ts">
	import Sms from './Sms.svelte';
	let { props }: { props: Record<string, unknown> } = $props();
</script>

<Sms direction="incoming" {props} />
