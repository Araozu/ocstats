<script lang="ts">
	import type { SessionDetail, Turn } from '$lib/api/ocstats';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { getModelPricingContext, type PricingRate } from '$lib/model-pricing';
	import {
		Table,
		TableBody,
		TableCell,
		TableHead,
		TableHeader,
		TableRow
	} from '$lib/components/ui/table';
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

	function turnCost(turn: Turn) {
		if (!turn.model) return null;
		let total = 0;
		for (const [tokens, rate] of [
			[turn.usage.input_tokens, 'input'],
			[turn.usage.cache_read_tokens, 'cached_read'],
			[turn.usage.output_tokens, 'output']
		] as const) {
			const cost = $pricingStore.cost(turn.model, tokens, rate);
			if (cost == null && tokens > 0) return null;
			total += cost ?? 0;
		}
		return total;
	}
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
	<Card>
		<CardHeader>
			<CardTitle>Turns</CardTitle>
			<p class="mt-1 text-xs text-muted-foreground">
				Token usage from each completed turn in this session.
			</p>
		</CardHeader>
		<CardContent class="p-0">
			<Table>
				<TableHeader>
					<TableRow>
						<TableHead class="pl-5">Turn</TableHead>
						<TableHead>Model</TableHead>
						<TableHead>Input</TableHead>
						<TableHead>Cached</TableHead>
						<TableHead>Output</TableHead>
						<TableHead class="pr-5 text-right">Pricing</TableHead>
					</TableRow>
				</TableHeader>
				<TableBody>
					{#each session.turns as turn, index (turn.id)}
						{@const model = turn.model}
						<TableRow>
							<TableCell class="pl-5 font-mono text-xs">{index + 1}</TableCell>
							<TableCell>
								{#if model}
									<p class="text-xs font-medium">{model.model_id}</p>
									<p class="text-[11px] text-muted-foreground">
										{model.provider_id}{model.variant ? ` · ${model.variant}` : ''}
									</p>
								{:else}
									<span class="text-xs text-muted-foreground">Unknown model</span>
								{/if}
							</TableCell>
							<TableCell>
								<p>{formatNumber(turn.usage.input_tokens)}</p>
								<p class="text-[11px] text-muted-foreground">
									{formatCost(
										model ? $pricingStore.cost(model, turn.usage.input_tokens, 'input') : null
									)}
								</p>
							</TableCell>
							<TableCell>
								<p>{formatNumber(turn.usage.cache_read_tokens)}</p>
								<p class="text-[11px] text-muted-foreground">
									{formatCost(
										model
											? $pricingStore.cost(model, turn.usage.cache_read_tokens, 'cached_read')
											: null
									)}
								</p>
							</TableCell>
							<TableCell>
								<p>{formatNumber(turn.usage.output_tokens)}</p>
								<p class="text-[11px] text-muted-foreground">
									{formatCost(
										model ? $pricingStore.cost(model, turn.usage.output_tokens, 'output') : null
									)}
								</p>
							</TableCell>
							<TableCell class="pr-5 text-right text-xs font-medium">
								{formatCost(turnCost(turn))}
							</TableCell>
						</TableRow>
					{:else}
						<TableRow>
							<TableCell colspan={6} class="h-24 text-center text-muted-foreground">
								No completed turns.
							</TableCell>
						</TableRow>
					{/each}
				</TableBody>
			</Table>
		</CardContent>
	</Card>
</div>
