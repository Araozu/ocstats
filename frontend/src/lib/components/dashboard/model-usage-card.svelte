<script lang="ts">
	import { requestPricing, type ModelUsage, type Usage } from '$lib/api/ocstats';
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';
	import { getModelPricingContext, type PricingRate } from '$lib/model-pricing';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { Tabs, TabsList, TabsTrigger } from '$lib/components/ui/tabs';
	import {
		Table,
		TableBody,
		TableCell,
		TableHead,
		TableHeader,
		TableRow
	} from '$lib/components/ui/table';
	import ModelPricingTooltip from './model-pricing-tooltip.svelte';
	import UsageCost from './usage-cost.svelte';
	import { formatCost, formatNumber } from './format';

	let {
		models,
		title = 'Models used',
		description = 'Token usage grouped by model from assistant messages.'
	}: {
		models: ModelUsage[];
		title?: string;
		description?: string;
	} = $props();
	const pricingStore = getModelPricingContext();
	const requestedPricing = new SvelteSet<string>();
	let grouping = $state('provider');
	const totalTokens = $derived(
		models.reduce((total, model) => total + (model.usage.total_tokens ?? 0), 0)
	);
	const totalCost = $derived($pricingStore.totalCost(models));

	function addUsage(left: Usage, right: Usage): Usage {
		return {
			cost: (left.cost ?? 0) + (right.cost ?? 0),
			input_tokens: left.input_tokens + right.input_tokens,
			output_tokens: left.output_tokens + right.output_tokens,
			reasoning_tokens: left.reasoning_tokens + right.reasoning_tokens,
			cache_read_tokens: left.cache_read_tokens + right.cache_read_tokens,
			cache_write_tokens: left.cache_write_tokens + right.cache_write_tokens,
			total_tokens: (left.total_tokens ?? 0) + (right.total_tokens ?? 0)
		};
	}

	function modelsCost(pricingModels: ModelUsage[], rate: PricingRate) {
		let total = 0;
		for (const model of pricingModels) {
			const tokens =
				rate === 'input'
					? model.usage.input_tokens
					: rate === 'cached_read'
						? model.usage.cache_read_tokens
						: rate === 'cached_write'
							? model.usage.cache_write_tokens
							: model.usage.output_tokens;
			const cost = $pricingStore.cost(model, tokens, rate);
			if (cost == null && tokens > 0) return null;
			total += cost ?? 0;
		}
		return total;
	}

	function tokenPercentage(tokens: number | null | undefined) {
		if (!totalTokens) return '0%';
		return new Intl.NumberFormat('en-US', {
			style: 'percent',
			maximumFractionDigits: 1
		}).format((tokens ?? 0) / totalTokens);
	}

	function costPercentage(cost: number | null) {
		if (cost == null || totalCost == null) return '—';
		if (!totalCost) return '0%';
		return new Intl.NumberFormat('en-US', {
			style: 'percent',
			maximumFractionDigits: 1
		}).format(cost / totalCost);
	}

	const modelRows = $derived.by(() => {
		if (grouping === 'provider') {
			return models.map((model) => ({ model, pricingModels: [model] }));
		}

		const grouped = new SvelteMap<string, { model: ModelUsage; pricingModels: ModelUsage[] }>();
		for (const model of models) {
			const existing = grouped.get(model.model_id);
			if (existing) {
				existing.model = { ...existing.model, usage: addUsage(existing.model.usage, model.usage) };
				existing.pricingModels.push(model);
			} else {
				grouped.set(model.model_id, { model: { ...model }, pricingModels: [model] });
			}
		}
		return [...grouped.values()];
	});

	$effect(() => {
		if (!$pricingStore.loaded) return;
		for (const model of models) {
			if ($pricingStore.find(model) || requestedPricing.has(model.model_id)) continue;
			requestedPricing.add(model.model_id);
			void requestPricing(model.model_id).catch(() => {
				requestedPricing.delete(model.model_id);
			});
		}
	});
</script>

<Card class="!overflow-visible">
	<CardHeader>
		<CardTitle>{title}</CardTitle>
		<p class="mt-1 text-xs text-muted-foreground">{description}</p>
		<Tabs bind:value={grouping} class="mt-3">
			<TabsList>
				<TabsTrigger value="provider">Per model + provider</TabsTrigger>
				<TabsTrigger value="model">Per model</TabsTrigger>
			</TabsList>
		</Tabs>
	</CardHeader>
	<CardContent class="p-0">
		<Table containerClass="!overflow-visible">
			<TableHeader>
				<TableRow>
					<TableHead class="pl-5">Model</TableHead>
					<TableHead>Input</TableHead>
					<TableHead>Cache read</TableHead>
					<TableHead>Cache write</TableHead>
					<TableHead>Output</TableHead>
					<TableHead class="pr-5 text-right">Total</TableHead>
				</TableRow>
			</TableHeader>
			<TableBody>
				{#each modelRows as { model, pricingModels } (`${grouping}:${model.provider_id}:${model.model_id}:${model.variant ?? ''}`)}
					<TableRow>
						<TableCell class="pl-5">
							<div class="flex items-center gap-1.5">
								<p class="text-xs font-medium">{model.model_id}</p>
								<ModelPricingTooltip
									model={pricingModels.length === 1 ? pricingModels[0] : undefined}
								/>
							</div>
							<p class="text-[11px] text-muted-foreground">
								{grouping === 'provider'
									? `${model.provider_id}${model.variant ? ` · ${model.variant}` : ''}`
									: 'All providers'}
							</p>
						</TableCell>
						<TableCell
							><UsageCost
								tokens={model.usage.input_tokens}
								cost={modelsCost(pricingModels, 'input')}
							/></TableCell
						>
						<TableCell
							><UsageCost
								tokens={model.usage.cache_read_tokens}
								cost={modelsCost(pricingModels, 'cached_read')}
							/></TableCell
						>
						<TableCell
							><UsageCost
								tokens={model.usage.cache_write_tokens}
								cost={modelsCost(pricingModels, 'cached_write')}
							/></TableCell
						>
						<TableCell class="pr-5"
							><UsageCost
								tokens={model.usage.output_tokens}
								cost={modelsCost(pricingModels, 'output')}
							/></TableCell
						>
						<TableCell class="pr-5 text-right text-xs font-medium">
							<p class="text-[11px] text-muted-foreground">
								{tokenPercentage(model.usage.total_tokens)}
							</p>
							<p>{formatNumber(model.usage.total_tokens)}</p>
							<p class="text-[11px] text-muted-foreground">
								{formatCost($pricingStore.totalCost(pricingModels))}, {costPercentage(
									$pricingStore.totalCost(pricingModels)
								)}
							</p>
						</TableCell>
					</TableRow>
				{:else}
					<TableRow>
						<TableCell colspan={6} class="h-24 text-center text-muted-foreground">
							No model usage records.
						</TableCell>
					</TableRow>
				{/each}
			</TableBody>
		</Table>
	</CardContent>
</Card>
