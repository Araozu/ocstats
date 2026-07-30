<script lang="ts">
	import type { Model, ModelUsage } from '$lib/api/ocstats';
	import InfoIcon from 'phosphor-svelte/lib/InfoIcon';
	import { SvelteSet } from 'svelte/reactivity';
	import * as Popover from '$lib/components/ui/popover';
	import { getModelPricingContext } from '$lib/model-pricing';
	import { formatPrice } from './format';

	let {
		model,
		models = [],
		atMs
	}: { model?: Model; models?: ModelUsage[]; atMs?: number } = $props();
	const pricingStore = getModelPricingContext();
	const pricingEntries = $derived.by(() => {
		const seen = new SvelteSet<string>();
		const candidates = models.length
			? models.map((candidate) => ({ model: candidate, atMs: candidate.created_at_ms }))
			: model && atMs !== undefined
				? [{ model, atMs }]
				: [];
		return candidates.flatMap(({ model: candidate, atMs: candidateAtMs }) => {
			const pricing = $pricingStore.find(candidate, candidateAtMs);
			const key = `${candidate.provider_id}\0${candidate.model_id}\0${pricing?.effective_from ?? 'unknown'}`;
			if (seen.has(key)) return [];
			seen.add(key);
			return [{ key, model: candidate, pricing }];
		});
	});
</script>

<Popover.Root>
	<Popover.Trigger
		class="inline-flex size-6 shrink-0 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50"
		aria-label="Show model pricing"
	>
		<InfoIcon size={13} />
	</Popover.Trigger>
	<Popover.Content sideOffset={6} class="block w-48 max-w-[calc(100vw-2rem)] p-3">
		<p class="font-medium">Price per 1M tokens</p>
		<div class="mt-2 space-y-3 text-[11px]">
			{#each pricingEntries as entry (entry.key)}
				<div>
					{#if pricingEntries.length > 1}
						<p class="font-medium text-muted-foreground">{entry.model.provider_id}</p>
					{/if}
					<div class="space-y-1">
						<div class="flex justify-between gap-4">
							<span>Input</span><span>{formatPrice(entry.pricing?.input)}</span>
						</div>
						<div class="flex justify-between gap-4">
							<span>Cached read</span><span>{formatPrice(entry.pricing?.cached_read)}</span>
						</div>
						<div class="flex justify-between gap-4">
							<span>Cached write</span><span>{formatPrice(entry.pricing?.cached_write)}</span>
						</div>
						<div class="flex justify-between gap-4">
							<span>Output</span><span>{formatPrice(entry.pricing?.output)}</span>
						</div>
					</div>
				</div>
			{/each}
		</div>
	</Popover.Content>
</Popover.Root>
