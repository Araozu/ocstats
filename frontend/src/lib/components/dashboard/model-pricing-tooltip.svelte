<script lang="ts">
	import type { Model } from '$lib/api/ocstats';
	import InfoIcon from 'phosphor-svelte/lib/InfoIcon';
	import { getModelPricingContext } from '$lib/model-pricing';
	import { formatPrice } from './format';

	let { model }: { model: Model | undefined } = $props();
	const pricingStore = getModelPricingContext();
	const pricing = $derived(model ? $pricingStore.find(model) : undefined);
</script>

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
	</span>
</button>
