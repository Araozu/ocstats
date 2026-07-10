<script lang="ts">
	import type { SessionDetail } from '$lib/api/ocstats';
	import ModelUsageCard from './model-usage-card.svelte';
	import SessionIdentity from './session-identity.svelte';
	import SessionMetrics from './session-metrics.svelte';
	import SessionTurns from './session-turns.svelte';

	let { session }: { session: SessionDetail } = $props();
</script>

<div class="space-y-7">
	<SessionIdentity {session} />
	<SessionMetrics {session} />
	<ModelUsageCard models={session.models} />
	<SessionTurns {session} />
</div>
