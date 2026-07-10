<script lang="ts">
	import type { ModelPricing, ModelUsage } from '$lib/api/ocstats';
	import InfoIcon from 'phosphor-svelte/lib/InfoIcon';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
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
		pricing,
		title = 'Models used',
		description = 'Token usage grouped by model from assistant messages.'
	}: {
		models: ModelUsage[];
		pricing: ModelPricing[];
		title?: string;
		description?: string;
	} = $props();

	const pricedModels = $derived.by(() => {
		const pricingByProviderAndSlug = new Map(
			pricing.map((item) => [`${item.provider}:${item.slug}`, item])
		);
		const pricingBySlug = new Map(pricing.map((item) => [item.slug, item]));

		return models.map((model) => ({
			model,
			pricing:
				pricingByProviderAndSlug.get(`${model.provider_id}:${model.model_id}`) ??
				pricingBySlug.get(model.model_id)
		}));
	});

	$effect(() => {
		if (!import.meta.env.DEV) return;

		console.info('[model-usage-card] pricing lookups', {
			pricingCount: pricing.length,
			lookups: pricedModels.map(({ model, pricing: modelPricing }) => ({
				provider: model.provider_id,
				model: model.model_id,
				matched: modelPricing ? `${modelPricing.provider}:${modelPricing.slug}` : null,
				input: modelPricing?.input ?? null,
				cachedRead: modelPricing?.cached_read ?? null,
				output: modelPricing?.output ?? null
			}))
		});
	});

	function tokenCost(tokens: number, price: number | null | undefined) {
		return price == null ? null : (tokens * price) / 1_000_000;
	}
</script>

<Card>
	<CardHeader>
		<CardTitle>{title}</CardTitle>
		<p class="mt-1 text-xs text-muted-foreground">{description}</p>
	</CardHeader>
	<CardContent class="p-0">
		<Table>
			<TableHeader>
				<TableRow>
					<TableHead class="pl-5">Model</TableHead>
					<TableHead>Input</TableHead>
					<TableHead>Cached</TableHead>
					<TableHead>Output</TableHead>
				</TableRow>
			</TableHeader>
			<TableBody>
				{#each pricedModels as { model, pricing: modelPricing } (`${model.provider_id}:${model.model_id}:${model.variant ?? ''}`)}
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
										class="pointer-events-none invisible absolute left-0 top-full z-50 mt-2 w-48 rounded-md bg-popover p-3 text-popover-foreground opacity-0 shadow-md ring-1 ring-foreground/10 transition-opacity group-hover:visible group-hover:opacity-100 group-focus-within:visible group-focus-within:opacity-100"
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
								{model.provider_id}{model.variant ? ` · ${model.variant}` : ''}
							</p>
						</TableCell>
						<TableCell>
							<p>{formatNumber(model.usage.input_tokens)}</p>
							<p class="text-[11px] text-muted-foreground">
								{formatCost(tokenCost(model.usage.input_tokens, modelPricing?.input))}
							</p>
						</TableCell>
						<TableCell>
							<p>{formatNumber(model.usage.cache_read_tokens)}</p>
							<p class="text-[11px] text-muted-foreground">
								{formatCost(tokenCost(model.usage.cache_read_tokens, modelPricing?.cached_read))}
							</p>
						</TableCell>
						<TableCell class="pr-5">
							<p>{formatNumber(model.usage.output_tokens)}</p>
							<p class="text-[11px] text-muted-foreground">
								{formatCost(tokenCost(model.usage.output_tokens, modelPricing?.output))}
							</p>
						</TableCell>
					</TableRow>
				{:else}
					<TableRow>
						<TableCell colspan={4} class="h-24 text-center text-muted-foreground">
							No model usage records.
						</TableCell>
					</TableRow>
				{/each}
			</TableBody>
		</Table>
	</CardContent>
</Card>
