<script lang="ts">
	import type { SessionDetail } from '$lib/api/ocstats';
	import { formatDateTime, shortId } from './format';

	let { session }: { session: SessionDetail } = $props();
</script>

<section>
	<h2 class="mt-2 break-words text-2xl font-semibold tracking-tight">
		{session.title || 'Untitled session'}
	</h2>
	<p class="mt-2 break-all font-mono text-xs text-muted-foreground">
		{session.source} · {shortId(session.session_id)}
	</p>
	<p class="mt-2 text-xs text-muted-foreground">
		Started
		<time datetime={new Date(session.created_at_ms).toISOString()}>
			{formatDateTime(session.created_at_ms)}
		</time>
	</p>
</section>
