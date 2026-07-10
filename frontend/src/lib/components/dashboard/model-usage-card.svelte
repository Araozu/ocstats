<script lang="ts">
	import { requestPricing, type ModelUsage, type Usage } from '$lib/api/ocstats';
	import InfoIcon from 'phosphor-svelte/lib/InfoIcon';
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
	import { formatCost, formatNumber, formatPrice } from './format';

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
	const requestedPricing = new Set<string>();
	let grouping = $state('provider');

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

	const modelRows = $derived.by(() => {
		if (grouping === 'provider') {
			return models.map((model) => ({ model, pricingModels: [model] }));
		}

		const grouped = new Map<string, { model: ModelUsage; pricingModels: ModelUsage[] }>();
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
					<TableHead>Cached</TableHead>
					<TableHead>Output</TableHead>
					<TableHead class="pr-5 text-right">Total</TableHead>
				</TableRow>
			</TableHeader>
			<TableBody>
				{#each modelRows as { model, pricingModels } (`${grouping}:${model.provider_id}:${model.model_id}:${model.variant ?? ''}`)}
					{@const modelPricing =
						pricingModels.length === 1 ? $pricingStore.find(pricingModels[0]) : undefined}
					<TableRow>
						<TableCell class="pl-5">
							<div class="flex items-center gap-1.5">
								<p class="text-xs font-medium">{model.model_id}</p>
								<button
									type="button"
									class="group relative inline-flex border-0 bg-transparent p-0 text-left"
									aria-label="Show model pricing"
								>
									<InfoIcon size={13} class="text-muted-foreground" />
									<span
										role="dialog"
										class="pointer-events-none invisible absolute left-full top-1/2 z-50 ml-2 w-48 -translate-y-1/2 rounded-md bg-popover p-3 text-popover-foreground opacity-0 shadow-md ring-1 ring-foreground/10 transition-opacity group-hover:visible group-hover:opacity-100 group-focus-within:visible group-focus-within:opacity-100"
									>
										<p class="font-medium">Price per 1M tokens</p>
										<div class="mt-2 space-y-1 text-[11px]">
											<div class="flex justify-between gap-4">
												<span>Input</span><span>{formatPrice(modelPricing?.input)}</span>
											</div>
											<div class="flex justify-between gap-4">
												<span>Cached read</span><span>{formatPrice(modelPricing?.cached_read)}</span
												>
											</div>
											<div class="flex justify-between gap-4">
												<span>Cached write</span><span
													>{formatPrice(modelPricing?.cached_write)}</span
												>
											</div>
											<div class="flex justify-between gap-4">
												<span>Output</span><span>{formatPrice(modelPricing?.output)}</span>
											</div>
										</div>
									</span>
								</button>
							</div>
							<p class="text-[11px] text-muted-foreground">
								{grouping === 'provider'
									? `${model.provider_id}${model.variant ? ` · ${model.variant}` : ''}`
									: 'All providers'}
							</p>
						</TableCell>
						<TableCell>
							<p>{formatNumber(model.usage.input_tokens)}</p>
							<p class="text-[11px] text-muted-foreground">
								{formatCost(modelsCost(pricingModels, 'input'))}
							</p>
						</TableCell>
						<TableCell>
							<p>{formatNumber(model.usage.cache_read_tokens)}</p>
							<p class="text-[11px] text-muted-foreground">
								{formatCost(modelsCost(pricingModels, 'cached_read'))}
							</p>
						</TableCell>
						<TableCell class="pr-5">
							<p>{formatNumber(model.usage.output_tokens)}</p>
							<p class="text-[11px] text-muted-foreground">
								{formatCost(modelsCost(pricingModels, 'output'))}
							</p>
						</TableCell>
						<TableCell class="pr-5 text-right text-xs font-medium">
							<p>{formatNumber(model.usage.total_tokens)}</p>
							<p class="text-[11px] text-muted-foreground">
								{formatCost($pricingStore.totalCost(pricingModels))}
							</p>
						</TableCell>
					</TableRow>
				{:else}
					<TableRow>
						<TableCell colspan={5} class="h-24 text-center text-muted-foreground">
							No model usage records.
						</TableCell>
					</TableRow>
				{/each}
			</TableBody>
		</Table>
	</CardContent>
</Card>
