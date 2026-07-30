<script lang="ts">
	import { requestPricing, type ModelUsage } from '$lib/api/ocstats';
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';
	import { getModelPricingContext, type PricingRate } from '$lib/model-pricing';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { Tabs, TabsList, TabsTrigger } from '$lib/components/ui/tabs';
	import {
		Table,
		TableBody,
		TableCell,
		TableFooter,
		TableHead,
		TableHeader,
		TableRow
	} from '$lib/components/ui/table';
	import ModelPricingTooltip from './model-pricing-tooltip.svelte';
	import UsageCost from './usage-cost.svelte';
	import { addUsage, emptyUsage, formatCost, formatNumber } from './format';

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

	const totalUsage = $derived(
		models.reduce((total, model) => addUsage(total, model.usage), emptyUsage)
	);

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
							: rate === 'reasoning'
								? model.usage.reasoning_tokens
								: model.usage.output_tokens;
			const cost = $pricingStore.cost(model, tokens, rate, model.created_at_ms);
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
		const grouped = new SvelteMap<string, { model: ModelUsage; pricingModels: ModelUsage[] }>();
		for (const model of models) {
			const key =
				grouping === 'provider'
					? `${model.provider_id}\0${model.model_id}\0${model.variant ?? ''}`
					: model.model_id;
			const existing = grouped.get(key);
			if (existing) {
				existing.model = { ...existing.model, usage: addUsage(existing.model.usage, model.usage) };
				existing.pricingModels.push(model);
			} else {
				grouped.set(key, { model: { ...model }, pricingModels: [model] });
			}
		}
		return [...grouped.values()];
	});

	$effect(() => {
		if (!$pricingStore.loaded) return;
		for (const model of models) {
			if (model.provider_id === 'unknown' || model.model_id === 'unknown') continue;
			if ($pricingStore.history(model) || requestedPricing.has(model.model_id)) continue;
			requestedPricing.add(model.model_id);
			void requestPricing(model.model_id).catch(() => {
				requestedPricing.delete(model.model_id);
			});
		}
	});
</script>

<Card>
	<CardHeader>
		<CardTitle>{title}</CardTitle>
		<p class="mt-1 text-xs text-muted-foreground">{description}</p>
		<Tabs bind:value={grouping} class="mt-3 max-w-full overflow-x-auto pb-1">
			<TabsList class="max-w-full">
				<TabsTrigger value="provider">Per model + provider</TabsTrigger>
				<TabsTrigger value="model">Per model</TabsTrigger>
			</TabsList>
		</Tabs>
	</CardHeader>
	<CardContent class="p-0">
		<Table class="min-w-[42rem]">
			<TableHeader>
				<TableRow>
					<TableHead class="pl-5">Model</TableHead>
					<TableHead>Input</TableHead>
					<TableHead>Cache read</TableHead>
					<TableHead>Cache write</TableHead>
					<TableHead>Reasoning</TableHead>
					<TableHead>Output</TableHead>
					<TableHead class="pr-5 text-right">Total</TableHead>
				</TableRow>
			</TableHeader>
			<TableBody>
				{#each modelRows as { model, pricingModels } (`${grouping}:${model.provider_id}:${model.model_id}:${model.variant ?? ''}`)}
					<TableRow>
						<TableCell class="w-52 max-w-52 pl-5">
							<div class="flex min-w-0 items-center gap-1.5">
								<p class="min-w-0 truncate text-xs font-medium" title={model.model_id}>
									{model.model_id}
								</p>
								<ModelPricingTooltip
									models={pricingModels}
									atMs={pricingModels.length === 1 ? pricingModels[0].created_at_ms : undefined}
								/>
							</div>
							<p class="truncate text-[11px] text-muted-foreground">
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
						<TableCell
							><UsageCost
								tokens={model.usage.reasoning_tokens}
								cost={modelsCost(pricingModels, 'reasoning')}
							/></TableCell
						>
						<TableCell class="pr-5"
							><UsageCost
								tokens={model.usage.output_tokens}
								cost={modelsCost(pricingModels, 'output')}
							/></TableCell
						>
						<TableCell class="pr-5 text-right text-xs font-medium">
							<p>
								{formatNumber(model.usage.total_tokens)} -
								<span title="tokens %" class="opacity-75">
									{tokenPercentage(model.usage.total_tokens)}
								</span>
							</p>
							<p class="text-[11px] text-muted-foreground">
								{formatCost($pricingStore.totalCost(pricingModels))} -
								<span title="price %" class="opacity-75">
									{costPercentage($pricingStore.totalCost(pricingModels))}
								</span>
							</p>
						</TableCell>
					</TableRow>
				{:else}
					<TableRow>
						<TableCell colspan={7} class="h-24 text-center text-muted-foreground">
							No model usage records.
						</TableCell>
					</TableRow>
				{/each}
			</TableBody>
			{#if models.length > 0}
				<TableFooter>
					<TableRow>
						<TableCell class="pl-5">Total</TableCell>
						<TableCell>
							<p>
								{formatNumber(totalUsage.input_tokens)} -
								<span title="tokens %" class="opacity-75">
									{tokenPercentage(totalUsage.input_tokens)}
								</span>
							</p>
							<p class="text-[11px] text-muted-foreground">
								{formatCost(modelsCost(models, 'input'))} -
								<span title="price %" class="opacity-75">
									{costPercentage(modelsCost(models, 'input'))}
								</span>
							</p>
						</TableCell>
						<TableCell>
							<p>
								{formatNumber(totalUsage.cache_read_tokens)} -
								<span title="tokens %" class="opacity-75">
									{tokenPercentage(totalUsage.cache_read_tokens)}
								</span>
							</p>
							<p class="text-[11px] text-muted-foreground">
								{formatCost(modelsCost(models, 'cached_read'))} -
								<span title="price %" class="opacity-75">
									{costPercentage(modelsCost(models, 'cached_read'))}
								</span>
							</p>
						</TableCell>
						<TableCell>
							<p>
								{formatNumber(totalUsage.cache_write_tokens)} -
								<span title="tokens %" class="opacity-75">
									{tokenPercentage(totalUsage.cache_write_tokens)}
								</span>
							</p>
							<p class="text-[11px] text-muted-foreground">
								{formatCost(modelsCost(models, 'cached_write'))} -
								<span title="price %" class="opacity-75">
									{costPercentage(modelsCost(models, 'cached_write'))}
								</span>
							</p>
						</TableCell>
						<TableCell>
							<p>
								{formatNumber(totalUsage.reasoning_tokens)} -
								<span title="tokens %" class="opacity-75">
									{tokenPercentage(totalUsage.reasoning_tokens)}
								</span>
							</p>
							<p class="text-[11px] text-muted-foreground">
								{formatCost(modelsCost(models, 'reasoning'))} -
								<span title="price %" class="opacity-75">
									{costPercentage(modelsCost(models, 'reasoning'))}
								</span>
							</p>
						</TableCell>
						<TableCell>
							<p>
								{formatNumber(totalUsage.output_tokens)} -
								<span title="tokens %" class="opacity-75">
									{tokenPercentage(totalUsage.output_tokens)}
								</span>
							</p>
							<p class="text-[11px] text-muted-foreground">
								{formatCost(modelsCost(models, 'output'))} -
								<span title="price %" class="opacity-75">
									{costPercentage(modelsCost(models, 'output'))}
								</span>
							</p>
						</TableCell>
						<TableCell class="pr-5 text-right">
							<p>
								{formatNumber(totalUsage.total_tokens)} -
								<span title="tokens %" class="opacity-75">
									{tokenPercentage(totalUsage.total_tokens)}
								</span>
							</p>
							<p class="text-[11px] text-muted-foreground">
								{formatCost(totalCost)} -
								<span title="price %" class="opacity-75">
									{costPercentage(totalCost)}
								</span>
							</p>
						</TableCell>
					</TableRow>
				</TableFooter>
			{/if}
		</Table>
	</CardContent>
</Card>
