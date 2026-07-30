<script lang="ts">
	import type { Model, ModelUsage } from '$lib/api/ocstats';
	import InfoIcon from 'phosphor-svelte/lib/InfoIcon';
	import { SvelteSet } from 'svelte/reactivity';
	import * as Popover from '$lib/components/ui/popover';
	import { getModelPricingContext } from '$lib/model-pricing';
	import { formatDateTime, formatPrice } from './format';

	let {
		model,
		models = [],
		atMs
	}: { model?: Model; models?: ModelUsage[]; atMs?: number } = $props();
	const pricingStore = getModelPricingContext();
	const pricingEntries = $derived.by(() => {
		const seen = new SvelteSet<string>();
		const candidates = models.length ? models : model ? [model] : [];
		return candidates.flatMap((candidate) => {
			const key = `${candidate.provider_id}\0${candidate.model_id}`;
			if (seen.has(key)) return [];
			seen.add(key);
			return [{ key, model: candidate, history: $pricingStore.history(candidate) }];
		});
	});
	const pricing = $derived(
		pricingEntries[0]
			? atMs === undefined
				? pricingEntries[0].history?.prices?.at(-1)
				: $pricingStore.find(pricingEntries[0].model, atMs)
			: undefined
	);
	const hasHistoricalPrices = $derived(
		pricingEntries.some((entry) => (entry.history?.prices?.length ?? 0) > 1)
	);
</script>

<Popover.Root>
	<Popover.Trigger
		class="inline-flex size-6 shrink-0 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50"
		aria-label="Show model pricing"
	>
		<InfoIcon size={13} />
	</Popover.Trigger>
	<Popover.Content sideOffset={6} class="block w-48 max-w-[calc(100vw-2rem)] p-3">
		{#if atMs === undefined && (pricingEntries.length > 1 || hasHistoricalPrices)}
			<p class="font-medium">Historical prices per 1M tokens</p>
			<div class="mt-2 space-y-3 text-[11px]">
				{#each pricingEntries as entry (entry.key)}
					<div>
						<p class="font-medium text-muted-foreground">{entry.model.provider_id}</p>
						{#each entry.history?.prices ?? [] as period (period.effective_from)}
							<div class="mt-2">
								<p class="font-medium text-muted-foreground">
									From {formatDateTime(Date.parse(period.effective_from))}
								</p>
								<div class="mt-1 space-y-1">
									<div class="flex justify-between gap-4">
										<span>Input</span><span>{formatPrice(period.input)}</span>
									</div>
									<div class="flex justify-between gap-4">
										<span>Cached read</span><span>{formatPrice(period.cached_read)}</span>
									</div>
									<div class="flex justify-between gap-4">
										<span>Cached write</span><span>{formatPrice(period.cached_write)}</span>
									</div>
									<div class="flex justify-between gap-4">
										<span>Output</span><span>{formatPrice(period.output)}</span>
									</div>
								</div>
							</div>
						{/each}
					</div>
				{/each}
			</div>
		{:else}
			<p class="font-medium">Price per 1M tokens</p>
			<div class="mt-2 space-y-1 text-[11px]">
				<div class="flex justify-between gap-4">
					<span>Input</span><span>{formatPrice(pricing?.input)}</span>
				</div>
				<div class="flex justify-between gap-4">
					<span>Cached read</span><span>{formatPrice(pricing?.cached_read)}</span>
				</div>
				<div class="flex justify-between gap-4">
					<span>Cached write</span><span>{formatPrice(pricing?.cached_write)}</span>
				</div>
				<div class="flex justify-between gap-4">
					<span>Output</span><span>{formatPrice(pricing?.output)}</span>
				</div>
			</div>
		{/if}
	</Popover.Content>
</Popover.Root>
