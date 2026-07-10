<script lang="ts">
	import type { ModelPricing, SessionDetail } from '$lib/api/ocstats';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import {
		Table,
		TableBody,
		TableCell,
		TableHead,
		TableHeader,
		TableRow
	} from '$lib/components/ui/table';
	import { formatCost, formatNumber, formatPrice, shortId } from './format';

	let {
		session,
		pricing,
		onBack
	}: { session: SessionDetail; pricing: ModelPricing[]; onBack: () => void } = $props();

	const pricingByModel = $derived(
		new Map(pricing.map((item) => [`${item.provider}:${item.slug}`, item]))
	);

	function findPricing(provider: string, slug: string) {
		return pricingByModel.get(`${provider}:${slug}`);
	}

	function metricCost(kind: 'input' | 'cached_read' | 'reasoning' | 'output') {
		return session.models.reduce((total, model) => {
			const modelPricing = findPricing(model.provider_id, model.model_id);
			if (!modelPricing) return total;

			const tokens =
				kind === 'cached_read' ? model.usage.cache_read_tokens : model.usage[`${kind}_tokens`];
			const price =
				kind === 'input'
					? modelPricing.input
					: kind === 'cached_read'
						? modelPricing.cached_read
						: modelPricing.output;
			return total + (price == null ? 0 : (tokens * price) / 1_000_000);
		}, 0);
	}
</script>

<div class="space-y-7">
	<div class="flex flex-wrap items-start justify-between gap-4">
		<div>
			<Button variant="ghost" size="sm" class="-ml-2 mb-3" onclick={onBack}
				>← Back to overview</Button
			>
			<p class="text-xs font-medium uppercase tracking-[0.12em] text-muted-foreground">
				Selected session
			</p>
			<h2 class="mt-2 text-2xl font-semibold tracking-tight">
				{session.title || 'Untitled session'}
			</h2>
			<p class="mt-2 font-mono text-xs text-muted-foreground">
				{session.source} · {shortId(session.session_id)}
			</p>
		</div>
		<Badge variant="secondary">{session.source_kind}</Badge>
	</div>
	<section class="grid gap-3 sm:grid-cols-2 xl:grid-cols-5">
		{#each [{ label: 'Input tokens', value: session.usage.input_tokens, cost: metricCost('input') }, { label: 'Cached tokens', value: session.usage.cache_read_tokens, cost: metricCost('cached_read') }, { label: 'Reasoning', value: session.usage.reasoning_tokens, cost: metricCost('reasoning') }, { label: 'Output tokens', value: session.usage.output_tokens, cost: metricCost('output') }, { label: 'Total cost', value: formatCost(session.usage.cost) }] as metric (metric.label)}
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
	<Card
		><CardHeader
			><CardTitle>Models used</CardTitle>
			<p class="mt-1 text-xs text-muted-foreground">
				Token usage grouped by model from assistant messages.
			</p></CardHeader
		><CardContent class="p-0"
			><Table
				><TableHeader
					><TableRow
						><TableHead class="pl-5">Model</TableHead><TableHead>Input</TableHead><TableHead
							>Cached</TableHead
						><TableHead>Reasoning</TableHead><TableHead>Output</TableHead><TableHead
							>Input price</TableHead
						><TableHead>Cache write</TableHead><TableHead>Cache read</TableHead><TableHead
							class="pr-5">Output price</TableHead
						></TableRow
					></TableHeader
				><TableBody
					>{#each session.models as model (model.provider_id + model.model_id + model.variant)}<TableRow
							><TableCell class="pl-5"
								><p class="text-xs font-medium">{model.model_id}</p>
								<p class="text-[11px] text-muted-foreground">
									{model.provider_id}{model.variant ? ` · ${model.variant}` : ''}
								</p></TableCell
							><TableCell>{formatNumber(model.usage.input_tokens)}</TableCell><TableCell
								>{formatNumber(model.usage.cache_read_tokens)}</TableCell
							><TableCell>{formatNumber(model.usage.reasoning_tokens)}</TableCell><TableCell
								>{formatNumber(model.usage.output_tokens)}</TableCell
							><TableCell
								>{formatPrice(findPricing(model.provider_id, model.model_id)?.input)}</TableCell
							><TableCell
								>{formatPrice(
									findPricing(model.provider_id, model.model_id)?.cached_write
								)}</TableCell
							><TableCell
								>{formatPrice(
									findPricing(model.provider_id, model.model_id)?.cached_read
								)}</TableCell
							><TableCell class="pr-5"
								>{formatPrice(findPricing(model.provider_id, model.model_id)?.output)}</TableCell
							></TableRow
						>{:else}<TableRow
							><TableCell colspan={9} class="h-24 text-center text-muted-foreground"
								>No model usage records.</TableCell
							></TableRow
						>{/each}</TableBody
				></Table
			></CardContent
		></Card
	>
</div>
