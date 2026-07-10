<script lang="ts">
	import type { Turn } from '$lib/api/ocstats';
	import { TableCell, TableRow } from '$lib/components/ui/table';
	import { getModelPricingContext } from '$lib/model-pricing';
	import ModelPricingTooltip from './model-pricing-tooltip.svelte';
	import UsageCost from './usage-cost.svelte';
	import { formatCost } from './format';

	let { turn, index }: { turn: Turn; index: number } = $props();
	const pricingStore = getModelPricingContext();
	const model = $derived(turn.model);
	function totalCost() {
		if (!model) return null;
		let total = 0;
		for (const [tokens, rate] of [
			[turn.usage.input_tokens, 'input'],
			[turn.usage.cache_read_tokens, 'cached_read'],
			[turn.usage.output_tokens, 'output']
		] as const) {
			const cost = $pricingStore.cost(model, tokens, rate);
			if (cost == null && tokens > 0) return null;
			total += cost ?? 0;
		}
		return total;
	}
</script>

<TableRow>
	<TableCell class="hidden pl-5 font-mono text-xs sm:table-cell">{index + 1}</TableCell>
	<TableCell>
		{#if model}<div class="flex items-center gap-1.5">
				<p class="text-xs font-medium">{model.model_id}</p>
				<ModelPricingTooltip {model} />
			</div>
			<p class="text-[11px] text-muted-foreground">
				{model.provider_id}{model.variant ? ` · ${model.variant}` : ''}
			</p>{:else}<span class="text-xs text-muted-foreground">Unknown model</span>{/if}
		<p class="mt-1 truncate text-[11px] text-muted-foreground sm:hidden">
			{turn.types.join(' + ') || 'No activity'}
		</p>
	</TableCell>
	<TableCell class="hidden max-w-48 sm:table-cell"
		><p class="truncate text-xs">{turn.types.join(' + ') || '—'}</p>
		{#if turn.reason}<p class="text-[11px] text-muted-foreground">{turn.reason}</p>{/if}</TableCell
	>
	<TableCell
		><UsageCost
			tokens={turn.usage.input_tokens}
			cost={model ? $pricingStore.cost(model, turn.usage.input_tokens, 'input') : null}
		/></TableCell
	>
	<TableCell class="hidden sm:table-cell"
		><UsageCost
			tokens={turn.usage.cache_read_tokens}
			cost={model ? $pricingStore.cost(model, turn.usage.cache_read_tokens, 'cached_read') : null}
		/></TableCell
	>
	<TableCell
		><UsageCost
			tokens={turn.usage.output_tokens}
			cost={model ? $pricingStore.cost(model, turn.usage.output_tokens, 'output') : null}
		/></TableCell
	>
	<TableCell class="pr-5 text-right text-xs font-medium">{formatCost(totalCost())}</TableCell>
</TableRow>
