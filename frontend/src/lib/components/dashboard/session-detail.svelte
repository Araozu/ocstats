<script lang="ts">
	import type { SessionDetail } from '$lib/api/ocstats';
	import { Card, CardContent } from '$lib/components/ui/card';
	import { getModelPricingContext, type PricingRate } from '$lib/model-pricing';
	import ModelUsageCard from './model-usage-card.svelte';
	import { formatCost, formatNumber, shortId } from './format';

	let { session }: { session: SessionDetail } = $props();
	const pricingStore = getModelPricingContext();

	function metricCost(kind: 'input' | 'cached_read' | 'reasoning' | 'output') {
		return session.models.reduce((total, model) => {
			const tokens =
				kind === 'cached_read' ? model.usage.cache_read_tokens : model.usage[`${kind}_tokens`];
			const rate: PricingRate =
				kind === 'input' ? 'input' : kind === 'cached_read' ? 'cached_read' : 'output';
			return total + ($pricingStore.cost(model, tokens, rate) ?? 0);
		}, 0);
	}

	const totalCost = $derived(
		metricCost('input') + metricCost('cached_read') + metricCost('reasoning') + metricCost('output')
	);
</script>

<div class="space-y-7">
	<div class="flex flex-wrap items-start justify-between gap-5">
		<div>
			<h2 class="mt-2 text-2xl font-semibold tracking-tight">
				{session.title || 'Untitled session'}
			</h2>
			<p class="mt-2 font-mono text-xs text-muted-foreground">
				{session.source} · {shortId(session.session_id)}
			</p>
		</div>
	</div>
	<section class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
		{#each [{ label: 'Input tokens', value: session.usage.input_tokens, cost: metricCost('input') }, { label: 'Cached tokens', value: session.usage.cache_read_tokens, cost: metricCost('cached_read') }, { label: 'Output tokens', value: session.usage.output_tokens, cost: metricCost('output') }, { label: 'Total cost', value: formatCost(totalCost) }] as metric (metric.label)}
			<Card size="sm">
				<CardContent class="p-4">
					<p class="text-xs text-muted-foreground">{metric.label}</p>
					<p class="mt-2 text-2xl font-semibold tracking-tight">
						{typeof metric.value === 'string' ? metric.value : formatNumber(metric.value)}
					</p>
				</CardContent>
				{#if metric.cost !== undefined}
					<p class="px-4 pb-4 text-xs text-muted-foreground">{formatCost(metric.cost)}</p>
				{/if}
			</Card>
		{/each}
	</section>
	<ModelUsageCard models={session.models} />
</div>
