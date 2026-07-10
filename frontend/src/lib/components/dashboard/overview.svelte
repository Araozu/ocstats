<script lang="ts">
	import ArrowUpRightIcon from 'phosphor-svelte/lib/ArrowUpRightIcon';
	import CoinsIcon from 'phosphor-svelte/lib/CoinsIcon';
	import HashIcon from 'phosphor-svelte/lib/HashIcon';
	import LightningIcon from 'phosphor-svelte/lib/LightningIcon';
	import TrendUpIcon from 'phosphor-svelte/lib/TrendUpIcon';
	import type { SessionUsage, Usage } from '$lib/api/ocstats';
	import { Badge } from '$lib/components/ui/badge';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { Separator } from '$lib/components/ui/separator';
	import {
		Table,
		TableBody,
		TableCell,
		TableHead,
		TableHeader,
		TableRow
	} from '$lib/components/ui/table';
	import { formatCost, formatNumber, sessionKey, shortId } from './format';

	let {
		projectName,
		sessions,
		modelCount,
		totals,
		isLoading = false,
		onSessionSelect
	}: {
		projectName: string;
		sessions: SessionUsage[];
		modelCount: number;
		totals: Usage;
		isLoading?: boolean;
		onSessionSelect: (session: SessionUsage) => void;
	} = $props();
	const billedTokens = $derived(
		totals.input_tokens + totals.output_tokens + totals.reasoning_tokens
	);
	const percent = (value: number) =>
		billedTokens ? Math.min(100, (value / billedTokens) * 100) : 0;
</script>

<div class="space-y-7">
	<section class="flex flex-wrap items-end justify-between gap-4">
		<div>
			<p class="text-xs font-medium uppercase tracking-[0.12em] text-muted-foreground">Scope</p>
			<h2 class="mt-1 text-2xl font-semibold tracking-tight">{projectName}</h2>
			<p class="mt-2 text-sm text-muted-foreground">
				A concise view of your recorded OpenCode usage.
			</p>
		</div>
		<Badge variant="outline">{sessions.length} sessions</Badge>
	</section>
	<section class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
		{#each [{ label: 'Total cost', value: formatCost(totals.cost), detail: 'Across this selection', icon: CoinsIcon }, { label: 'Total tokens', value: formatNumber(totals.total_tokens), detail: 'All reported tokens', icon: HashIcon }, { label: 'Sessions', value: formatNumber(sessions.length), detail: 'Recorded conversations', icon: LightningIcon }, { label: 'Models', value: formatNumber(modelCount), detail: 'Available models', icon: TrendUpIcon }] as metric (metric.label)}
			<Card size="sm"
				><CardContent class="flex items-start justify-between p-5"
					><div>
						<p class="text-xs text-muted-foreground">{metric.label}</p>
						<p class="mt-2 text-2xl font-semibold tracking-tight">{metric.value}</p>
						<p class="mt-1 text-[11px] text-muted-foreground">{metric.detail}</p>
					</div>
					<div class="rounded-md bg-muted p-2 text-muted-foreground">
						<metric.icon size={17} />
					</div></CardContent
				></Card
			>
		{/each}
	</section>
	<section class="grid gap-5 xl:grid-cols-[minmax(0,1.45fr)_minmax(17rem,0.55fr)]">
		<Card
			><CardHeader class="flex flex-row items-center justify-between space-y-0"
				><div>
					<CardTitle>Sessions</CardTitle>
					<p class="mt-1 text-xs text-muted-foreground">
						Select a session to inspect its model usage.
					</p>
				</div>
				<ArrowUpRightIcon size={17} class="text-muted-foreground" /></CardHeader
			><CardContent class="p-0"
				><Table
					><TableHeader
						><TableRow
							><TableHead class="pl-5">Session</TableHead><TableHead>Source</TableHead><TableHead
								>Tokens</TableHead
							><TableHead class="pr-5 text-right">Cost</TableHead></TableRow
						></TableHeader
					><TableBody
						>{#each sessions.slice(0, 12) as session (sessionKey(session.source, session.session_id))}<TableRow
								class="cursor-pointer"
								onclick={() => onSessionSelect(session)}
								><TableCell class="pl-5"
									><p class="max-w-64 truncate text-xs font-medium">
										{session.title || 'Untitled session'}
									</p>
									<p class="mt-0.5 font-mono text-[10px] text-muted-foreground">
										{shortId(session.session_id)}
									</p></TableCell
								><TableCell><Badge variant="secondary">{session.source_kind}</Badge></TableCell
								><TableCell>{formatNumber(session.usage.total_tokens)}</TableCell><TableCell
									class="pr-5 text-right font-medium">{formatCost(session.usage.cost)}</TableCell
								></TableRow
							>{:else}<TableRow
								><TableCell colspan={4} class="h-28 text-center text-muted-foreground"
									>{isLoading ? 'Loading usage...' : 'No usage data for this project.'}</TableCell
								></TableRow
							>{/each}</TableBody
					></Table
				></CardContent
			></Card
		>
		<Card
			><CardHeader
				><CardTitle>Token mix</CardTitle>
				<p class="mt-1 text-xs text-muted-foreground">Billable usage composition.</p></CardHeader
			><CardContent class="space-y-5"
				>{#each [{ label: 'Input', value: totals.input_tokens, color: 'bg-foreground' }, { label: 'Output', value: totals.output_tokens, color: 'bg-muted-foreground' }, { label: 'Reasoning', value: totals.reasoning_tokens, color: 'bg-primary' }] as item (item.label)}<div
					>
						<div class="mb-2 flex justify-between text-xs">
							<span>{item.label}</span><span class="text-muted-foreground"
								>{formatNumber(item.value)}</span
							>
						</div>
						<div class="h-2 overflow-hidden rounded-full bg-muted">
							<div
								class="h-full rounded-full {item.color}"
								style={`width: ${percent(item.value)}%`}
							></div>
						</div>
					</div>{/each}<Separator />
				<div class="space-y-3 text-xs">
					<div class="flex justify-between">
						<span class="text-muted-foreground">Cache read</span><span class="font-medium"
							>{formatNumber(totals.cache_read_tokens)}</span
						>
					</div>
					<div class="flex justify-between">
						<span class="text-muted-foreground">Cache write</span><span class="font-medium"
							>{formatNumber(totals.cache_write_tokens)}</span
						>
					</div>
				</div></CardContent
			></Card
		>
	</section>
</div>
