<script lang="ts">
	import type { SessionDetail, Turn } from '$lib/api/ocstats';
	import InfoIcon from 'phosphor-svelte/lib/InfoIcon';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { Button } from '$lib/components/ui/button';
	import { getModelPricingContext, type PricingRate } from '$lib/model-pricing';
	import {
		Table,
		TableBody,
		TableCell,
		TableHead,
		TableHeader,
		TableRow
	} from '$lib/components/ui/table';
	import ModelUsageCard from './model-usage-card.svelte';
	import { formatCost, formatNumber, formatPrice, shortId } from './format';

	let { session }: { session: SessionDetail } = $props();
	const pricingStore = getModelPricingContext();
	let expandedMessages = $state<string[]>([]);

	function toggleMessage(message: string) {
		expandedMessages = expandedMessages.includes(message)
			? expandedMessages.filter((item) => item !== message)
			: [...expandedMessages, message];
	}

	function metricCost(kind: 'input' | 'cached_read' | 'reasoning' | 'output') {
		return session.models.reduce((total, model) => {
			const tokens =
				kind === 'cached_read' ? model.usage.cache_read_tokens : model.usage[`${kind}_tokens`];
			const rate: PricingRate =
				kind === 'input' ? 'input' : kind === 'cached_read' ? 'cached_read' : 'output';
			return total + ($pricingStore.cost(model, tokens, rate) ?? 0);
		}, 0);
	}

	const totalCost = $derived(
		metricCost('input') + metricCost('cached_read') + metricCost('reasoning') + metricCost('output')
	);

	function turnCost(turn: Turn) {
		if (!turn.model) return null;
		let total = 0;
		for (const [tokens, rate] of [
			[turn.usage.input_tokens, 'input'],
			[turn.usage.cache_read_tokens, 'cached_read'],
			[turn.usage.output_tokens, 'output']
		] as const) {
			const cost = $pricingStore.cost(turn.model, tokens, rate);
			if (cost == null && tokens > 0) return null;
			total += cost ?? 0;
		}
		return total;
	}

	function userMessageCost(startIndex: number) {
		const message = session.turns[startIndex]?.user_message;
		if (!message) return null;

		let total = 0;
		for (let index = startIndex; index < session.turns.length; index += 1) {
			const turn = session.turns[index];
			if (turn.user_message !== message) break;
			const cost = turnCost(turn);
			if (cost == null) return null;
			total += cost;
		}
		return total;
	}
</script>

<div class="space-y-7">
	<div class="flex flex-wrap items-start justify-between gap-5">
		<div>
			<h2 class="mt-2 text-2xl font-semibold tracking-tight">
				{session.title || 'Untitled session'}
			</h2>
			<p class="mt-2 font-mono text-xs text-muted-foreground">
				{session.source} · {shortId(session.session_id)}
			</p>
		</div>
	</div>
	<section class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
		{#each [{ label: 'Input tokens', value: session.usage.input_tokens, cost: metricCost('input') }, { label: 'Cached tokens', value: session.usage.cache_read_tokens, cost: metricCost('cached_read') }, { label: 'Output tokens', value: session.usage.output_tokens, cost: metricCost('output') }, { label: 'Total cost', value: formatCost(totalCost) }] as metric (metric.label)}
			<Card size="sm">
				<CardContent class="p-4">
					<p class="text-xs text-muted-foreground">{metric.label}</p>
					<p class="mt-2 text-2xl font-semibold tracking-tight">
						{typeof metric.value === 'string' ? metric.value : formatNumber(metric.value)}
					</p>
				</CardContent>
				{#if metric.cost !== undefined}
					<p class="px-4 pb-4 text-xs text-muted-foreground">{formatCost(metric.cost)}</p>
				{/if}
			</Card>
		{/each}
	</section>
	<ModelUsageCard models={session.models} />
	<Card class="!overflow-visible">
		<CardHeader>
			<CardTitle>Turns</CardTitle>
			<p class="mt-1 text-xs text-muted-foreground">
				Token usage from each completed turn in this session.
			</p>
		</CardHeader>
		<CardContent class="p-0">
			<Table containerClass="!overflow-visible">
				<TableHeader>
					<TableRow>
						<TableHead class="pl-5">Turn</TableHead>
						<TableHead>Model</TableHead>
						<TableHead>Activity</TableHead>
						<TableHead>Input</TableHead>
						<TableHead>Cached</TableHead>
						<TableHead>Output</TableHead>
						<TableHead class="pr-5 text-right">Pricing</TableHead>
					</TableRow>
				</TableHeader>
				<TableBody>
					{#each session.turns as turn, index (turn.id)}
						{@const model = turn.model}
						{@const modelPricing = model ? $pricingStore.find(model) : undefined}
						{@const messageExpanded = turn.user_message
							? expandedMessages.includes(turn.user_message)
							: false}
						{#if turn.user_message && (index === 0 || session.turns[index - 1].user_message !== turn.user_message)}
							<TableRow class="bg-primary/15 hover:bg-primary/25">
								<TableCell colspan={7} class="px-5 py-3">
									<div class="flex items-center justify-between gap-4">
										<p
											class="text-[11px] font-medium uppercase tracking-wide text-muted-foreground"
										>
											User message
										</p>
										<p class="text-[11px] text-muted-foreground">
											<span class="font-medium uppercase tracking-wide">Summary</span>
											{formatCost(userMessageCost(index))}
										</p>
									</div>
									<p
										class="mt-1 whitespace-pre-wrap text-xs {messageExpanded ? '' : 'line-clamp-3'}"
									>
										{turn.user_message}
									</p>
									{#if turn.user_message.length > 240}
										<Button
											variant="ghost"
											size="xs"
											class="mt-2"
											onclick={() => toggleMessage(turn.user_message!)}
											aria-expanded={messageExpanded}
										>
											{messageExpanded ? 'Show less' : 'Show more'}
										</Button>
									{/if}
								</TableCell>
							</TableRow>
						{/if}
						<TableRow>
							<TableCell class="pl-5 font-mono text-xs">{index + 1}</TableCell>
							<TableCell>
								{#if model}
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
														<span>Cached read</span><span
															>{formatPrice(modelPricing?.cached_read)}</span
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
								{:else}
									<span class="text-xs text-muted-foreground">Unknown model</span>
								{/if}
							</TableCell>
							<TableCell class="max-w-48">
								<p class="truncate text-xs">{turn.types.join(' + ') || '—'}</p>
								{#if turn.reason}
									<p class="text-[11px] text-muted-foreground">{turn.reason}</p>
								{/if}
							</TableCell>
							<TableCell>
								<p>{formatNumber(turn.usage.input_tokens)}</p>
								<p class="text-[11px] text-muted-foreground">
									{formatCost(
										model ? $pricingStore.cost(model, turn.usage.input_tokens, 'input') : null
									)}
								</p>
							</TableCell>
							<TableCell>
								<p>{formatNumber(turn.usage.cache_read_tokens)}</p>
								<p class="text-[11px] text-muted-foreground">
									{formatCost(
										model
											? $pricingStore.cost(model, turn.usage.cache_read_tokens, 'cached_read')
											: null
									)}
								</p>
							</TableCell>
							<TableCell>
								<p>{formatNumber(turn.usage.output_tokens)}</p>
								<p class="text-[11px] text-muted-foreground">
									{formatCost(
										model ? $pricingStore.cost(model, turn.usage.output_tokens, 'output') : null
									)}
								</p>
							</TableCell>
							<TableCell class="pr-5 text-right text-xs font-medium">
								{formatCost(turnCost(turn))}
							</TableCell>
						</TableRow>
					{:else}
						<TableRow>
							<TableCell colspan={7} class="h-24 text-center text-muted-foreground">
								No completed turns.
							</TableCell>
						</TableRow>
					{/each}
				</TableBody>
			</Table>
		</CardContent>
	</Card>
</div>
