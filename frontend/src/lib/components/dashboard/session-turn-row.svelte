<script lang="ts">
	import { createQuery } from '@tanstack/svelte-query';
	import type { Turn } from '$lib/api/ocstats';
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDownIcon';
	import ArrowClockwiseIcon from 'phosphor-svelte/lib/ArrowClockwiseIcon';
	import { usageQueries } from '$lib/queries/usage';
	import { Button } from '$lib/components/ui/button';
	import { TableCell, TableRow } from '$lib/components/ui/table';
	import { getModelPricingContext } from '$lib/model-pricing';
	import ModelPricingTooltip from './model-pricing-tooltip.svelte';
	import UsageCost from './usage-cost.svelte';
	import { formatCost } from './format';

	let {
		turn,
		index,
		source,
		sessionId
	}: { turn: Turn; index: number; source: string; sessionId: string } = $props();
	let requested = $state(false);
	let expanded = $state(false);
	const textQuery = createQuery(() => usageQueries.turnText(source, sessionId, turn.id, requested));
	const detailVisible = $derived(expanded && (textQuery.data !== undefined || textQuery.isError));
	const detailId = $derived(`turn-text-${turn.id.replace(/[^a-zA-Z0-9_-]/g, '-')}`);
	const pricingStore = getModelPricingContext();
	const model = $derived(turn.model);
	function totalCost() {
		if (!model) return null;
		let total = 0;
		for (const [tokens, rate] of [
			[turn.usage.input_tokens, 'input'],
			[turn.usage.cache_read_tokens, 'cached_read'],
			[turn.usage.cache_write_tokens, 'cached_write'],
			[turn.usage.output_tokens, 'output']
		] as const) {
			const cost = $pricingStore.cost(model, tokens, rate);
			if (cost == null && tokens > 0) return null;
			total += cost ?? 0;
		}
		return total;
	}

	function activate() {
		if (requested && textQuery.isPending) return;
		if (textQuery.isError) {
			void textQuery.refetch();
			return;
		}
		if (!requested) requested = true;
		expanded = !expanded;
	}

	function partContent(data: unknown) {
		if (
			typeof data === 'object' &&
			data !== null &&
			'text' in data &&
			typeof data.text === 'string'
		) {
			return data.text;
		}
		return JSON.stringify(data, null, 2);
	}
</script>

<TableRow>
	<TableCell class="hidden pl-5 font-mono text-xs sm:table-cell">{index + 1}</TableCell>
	<TableCell class="w-44 max-w-44">
		{#if model}<div class="flex min-w-0 items-center gap-1.5">
				<p class="min-w-0 truncate text-xs font-medium" title={model.model_id}>{model.model_id}</p>
				<ModelPricingTooltip {model} />
			</div>
			<p class="truncate text-[11px] text-muted-foreground">
				{model.provider_id}{model.variant ? ` · ${model.variant}` : ''}
			</p>{:else}<span class="text-xs text-muted-foreground">Unknown model</span>{/if}
		<p class="mt-1 truncate text-[11px] text-muted-foreground sm:hidden">
			{turn.types.join(' + ') || 'No activity'}
		</p>
	</TableCell>
	<TableCell class="hidden max-w-48 sm:table-cell"
		><p class="truncate text-xs">{turn.types.join(' + ') || '—'}</p>
		{#if turn.reason}<p class="truncate text-[11px] text-muted-foreground">
				{turn.reason}
			</p>{/if}</TableCell
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
	<TableCell class="hidden sm:table-cell"
		><UsageCost
			tokens={turn.usage.cache_write_tokens}
			cost={model ? $pricingStore.cost(model, turn.usage.cache_write_tokens, 'cached_write') : null}
		/></TableCell
	>
	<TableCell
		><UsageCost
			tokens={turn.usage.output_tokens}
			cost={model ? $pricingStore.cost(model, turn.usage.output_tokens, 'output') : null}
		/></TableCell
	>
	<TableCell class="pr-5 text-right text-xs font-medium">{formatCost(totalCost())}</TableCell>
	<TableCell>
		<Button
			variant="ghost"
			size="xs"
			onclick={activate}
			disabled={requested && textQuery.isPending}
			aria-expanded={detailVisible}
			aria-controls={detailId}
		>
			{#if requested && textQuery.isPending}<ArrowClockwiseIcon
					class="animate-spin"
					aria-hidden="true"
				/>Loading{:else}<CaretDownIcon
					class={detailVisible ? 'rotate-180 transition-transform' : 'transition-transform'}
					aria-hidden="true"
				/>{textQuery.isError ? 'Retry' : detailVisible ? 'Hide text' : 'Show text'}{/if}
		</Button>
	</TableCell>
</TableRow>
{#if detailVisible}<TableRow id={detailId} class="bg-muted/30">
		<TableCell
			colspan={9}
			class="max-w-0 whitespace-pre-wrap break-words px-5 py-3 text-xs"
			aria-live="polite"
		>
			{#if textQuery.isError}<div class="flex items-center gap-2 text-destructive">
					<span>Unable to load turn text.</span><Button
						variant="ghost"
						size="xs"
						onclick={() => textQuery.refetch()}>Retry</Button
					>
				</div>
			{:else if textQuery.data?.parts === null}<span class="text-muted-foreground"
					>Output is unavailable for this imported turn. Import OpenCode data again.</span
				>
			{:else if textQuery.data?.parts?.length === 0}<span class="text-muted-foreground"
					>No output for this turn.</span
				>
			{:else}<div class="space-y-3">
					{#each textQuery.data?.parts ?? [] as part (part.id)}
						<section>
							<p
								class="mb-1 text-[10px] font-semibold uppercase tracking-wide text-muted-foreground"
							>
								{part.part_type}
							</p>
							<pre class="whitespace-pre-wrap break-all font-sans">{partContent(part.data)}</pre>
						</section>
					{/each}
				</div>{/if}
		</TableCell>
	</TableRow>{/if}
