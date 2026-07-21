<script lang="ts">
	import type { Model } from '$lib/api/ocstats';
	import InfoIcon from 'phosphor-svelte/lib/InfoIcon';
	import * as Popover from '$lib/components/ui/popover';
	import { getModelPricingContext } from '$lib/model-pricing';
	import { formatPrice } from './format';

	let { model }: { model: Model | undefined } = $props();
	const pricingStore = getModelPricingContext();
	const pricing = $derived(model ? $pricingStore.find(model) : undefined);
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
	</Popover.Content>
</Popover.Root>
