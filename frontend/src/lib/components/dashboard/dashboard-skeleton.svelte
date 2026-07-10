<script lang="ts">
	import { Card, CardContent } from '$lib/components/ui/card';
	let { rows = 4 }: { rows?: number } = $props();
</script>

<div class="space-y-7" aria-label="Loading dashboard" aria-busy="true">
	<section class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
		{#each Array.from({ length: 4 }, (_, index) => index) as index (index)}<Card size="sm"
				><CardContent class="space-y-3 p-4"
					><div class="h-3 w-20 animate-pulse rounded bg-muted"></div>
					<div class="h-7 w-28 animate-pulse rounded bg-muted"></div></CardContent
				></Card
			>{/each}
	</section>
	<Card
		><CardContent class="space-y-3 p-5"
			>{#each Array.from({ length: rows }, (_, index) => index) as index (index)}<div
					class="h-10 animate-pulse rounded bg-muted"
				></div>{/each}</CardContent
		></Card
	>
</div>
