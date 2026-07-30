<script lang="ts">
	import type { SessionDetail } from '$lib/api/ocstats';
	import { Card, CardContent } from '$lib/components/ui/card';
	import { getModelPricingContext, type PricingRate } from '$lib/model-pricing';
	import { formatCost, formatNumber } from './format';

	let { session }: { session: SessionDetail } = $props();
	const pricingStore = getModelPricingContext();

	function metricCost(kind: 'input' | 'cached_read' | 'cached_write' | 'reasoning' | 'output') {
		let total = 0;
		for (const model of session.models) {
			const tokens =
				kind === 'cached_read'
					? model.usage.cache_read_tokens
					: kind === 'cached_write'
						? model.usage.cache_write_tokens
						: model.usage[`${kind}_tokens`];
			const rate: PricingRate =
				kind === 'input'
					? 'input'
					: kind === 'cached_read'
						? 'cached_read'
						: kind === 'cached_write'
							? 'cached_write'
							: 'output';
			const cost = $pricingStore.cost(model, tokens, rate, model.created_at_ms);
			if (cost == null) return null;
			total += cost;
		}
		return total;
	}

	const totalCost = $derived.by(() => {
		const costs = [
			metricCost('input'),
			metricCost('cached_read'),
			metricCost('cached_write'),
			metricCost('reasoning'),
			metricCost('output')
		];
		let total = 0;
		for (const cost of costs) {
			if (cost == null) return null;
			total += cost;
		}
		return total;
	});
	const metrics = $derived([
		{ label: 'Input tokens', value: session.usage.input_tokens, cost: metricCost('input') },
		{
			label: 'Cache read tokens',
			value: session.usage.cache_read_tokens,
			cost: metricCost('cached_read')
		},
		{
			label: 'Cache write tokens',
			value: session.usage.cache_write_tokens,
			cost: metricCost('cached_write')
		},
		{ label: 'Output tokens', value: session.usage.output_tokens, cost: metricCost('output') },
		{ label: 'Total cost', value: formatCost(totalCost) }
	]);
</script>

<section class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
	{#each metrics as metric (metric.label)}
		<Card size="sm">
			<CardContent class="p-4">
				<p class="text-xs text-muted-foreground">{metric.label}</p>
				<p class="mt-2 break-all text-xl font-semibold tracking-tight sm:text-2xl">
					{typeof metric.value === 'string' ? metric.value : formatNumber(metric.value)}
				</p>
			</CardContent>
			{#if metric.cost !== undefined}
				<p class="px-4 pb-4 text-xs text-muted-foreground">{formatCost(metric.cost)}</p>
			{/if}
		</Card>
	{/each}
</section>
