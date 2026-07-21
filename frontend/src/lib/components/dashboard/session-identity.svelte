<script lang="ts">
	import type { SessionDetail } from '$lib/api/ocstats';
	import { shortId } from './format';

	let { session }: { session: SessionDetail } = $props();
</script>

<section>
	<h2 class="mt-2 break-words text-2xl font-semibold tracking-tight">
		{session.title || 'Untitled session'}
	</h2>
	<p class="mt-2 break-all font-mono text-xs text-muted-foreground">
		{session.source} · {shortId(session.session_id)}
	</p>
</section>
