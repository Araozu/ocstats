<script lang="ts">
	import ArrowsClockwise from 'phosphor-svelte/lib/ArrowsClockwiseIcon';
	import ChartLineUp from 'phosphor-svelte/lib/ChartLineUpIcon';
	import CircleNotch from 'phosphor-svelte/lib/CircleNotchIcon';
	import Coins from 'phosphor-svelte/lib/CoinsIcon';
	import Database from 'phosphor-svelte/lib/DatabaseIcon';
	import FolderSimple from 'phosphor-svelte/lib/FolderSimpleIcon';
	import Hash from 'phosphor-svelte/lib/HashIcon';
	import Lightning from 'phosphor-svelte/lib/LightningIcon';
	import TrendUp from 'phosphor-svelte/lib/TrendUpIcon';
	import WarningCircle from 'phosphor-svelte/lib/WarningCircleIcon';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Separator } from '$lib/components/ui/separator';
	import {
		Table,
		TableBody,
		TableCell,
		TableHead,
		TableHeader,
		TableRow
	} from '$lib/components/ui/table';

	type Project = { source: string; id: string; name: string | null; worktree: string };
	type Usage = {
		cost: number | null;
		input_tokens: number;
		output_tokens: number;
		reasoning_tokens: number;
		cache_read_tokens: number;
		cache_write_tokens: number;
		total_tokens: number | null;
	};
	type SessionUsage = { source: string; session_id: string; usage: Usage; source_kind: string };
	type Model = { provider_id: string; model_id: string; variant: string | null };
	type LoadState = 'loading' | 'ready' | 'error';

	const API_URL = 'http://127.0.0.1:4117/api';
	const emptyUsage: Usage = {
		cost: 0,
		input_tokens: 0,
		output_tokens: 0,
		reasoning_tokens: 0,
		cache_read_tokens: 0,
		cache_write_tokens: 0,
		total_tokens: 0
	};

	let projects = $state<Project[]>([]);
	let sessions = $state<SessionUsage[]>([]);
	let models = $state<Model[]>([]);
	let selectedProjectId = $state('all');
	let loadState = $state<LoadState>('loading');
	let errorMessage = $state('');
	let lastUpdated = $state<Date | null>(null);

	let selectedProject = $derived(
		projects.find((project) => project.id === selectedProjectId) ?? null
	);
	let visibleSessions = $derived(
		selectedProjectId === 'all'
			? sessions
			: sessions.filter((session) => session.source === selectedProject?.source)
	);
	let totals = $derived(
		visibleSessions.reduce((total, session) => addUsage(total, session.usage), emptyUsage)
	);
	let activeModels = $derived(models);

	async function loadDashboard() {
		loadState = 'loading';
		errorMessage = '';
		try {
			const [projectsResponse, sessionsResponse, modelsResponse] = await Promise.all([
				fetch(`${API_URL}/projects`),
				fetch(`${API_URL}/usage/sessions`),
				fetch(`${API_URL}/models`)
			]);
			if (![projectsResponse, sessionsResponse, modelsResponse].every((response) => response.ok)) {
				throw new Error('The analytics service returned an error.');
			}
			projects = await projectsResponse.json();
			sessions = await sessionsResponse.json();
			models = await modelsResponse.json();
			lastUpdated = new Date();
			loadState = 'ready';
		} catch (error) {
			loadState = 'error';
			errorMessage =
				error instanceof Error ? error.message : 'Could not connect to the analytics service.';
		}
	}

	function addUsage(left: Usage, right: Usage): Usage {
		return {
			cost: (left.cost ?? 0) + (right.cost ?? 0),
			input_tokens: left.input_tokens + right.input_tokens,
			output_tokens: left.output_tokens + right.output_tokens,
			reasoning_tokens: left.reasoning_tokens + right.reasoning_tokens,
			cache_read_tokens: left.cache_read_tokens + right.cache_read_tokens,
			cache_write_tokens: left.cache_write_tokens + right.cache_write_tokens,
			total_tokens: (left.total_tokens ?? 0) + (right.total_tokens ?? 0)
		};
	}

	function formatNumber(value: number | null | undefined) {
		return new Intl.NumberFormat('en-US', { notation: 'compact', maximumFractionDigits: 1 }).format(
			value ?? 0
		);
	}

	function formatCost(value: number | null | undefined) {
		return new Intl.NumberFormat('en-US', {
			style: 'currency',
			currency: 'USD',
			maximumFractionDigits: 2
		}).format(value ?? 0);
	}

	function projectLabel(project: Project) {
		return project.name?.trim() || project.worktree.split('/').filter(Boolean).pop() || project.id;
	}

	function shortId(value: string) {
		return value.length > 18 ? `${value.slice(0, 8)}...${value.slice(-6)}` : value;
	}

	$effect(() => {
		void loadDashboard();
	});
</script>

<svelte:head>
	<title>Usage overview | ocstats</title>
	<meta name="description" content="OpenCode usage and token statistics" />
</svelte:head>

<div class="min-h-screen bg-background text-foreground">
	<div class="flex min-h-screen flex-col md:flex-row">
		<aside
			class="w-full shrink-0 border-b bg-sidebar md:min-h-screen md:w-64 md:border-r md:border-b-0"
		>
			<div class="flex h-14 items-center gap-2 border-b px-4">
				<div
					class="flex size-7 items-center justify-center rounded-md bg-foreground text-background"
				>
					<ChartLineUp size={16} weight="bold" />
				</div>
				<div>
					<p class="text-sm font-semibold tracking-tight">ocstats</p>
					<p class="text-[11px] text-muted-foreground">OpenCode analytics</p>
				</div>
			</div>
			<div class="p-3">
				<div class="mb-2 flex items-center justify-between px-2">
					<p class="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
						Projects
					</p>
					<Badge variant="secondary">{projects.length}</Badge>
				</div>
				<ScrollArea class="h-auto max-h-64 md:max-h-[calc(100vh-8rem)]">
					<div class="space-y-0.5">
						<button
							class="flex w-full items-center gap-2 rounded-md px-2 py-2 text-left text-xs transition-colors hover:bg-sidebar-accent {selectedProjectId ===
							'all'
								? 'bg-sidebar-accent font-medium'
								: 'text-muted-foreground'}"
							onclick={() => (selectedProjectId = 'all')}
						>
							<Database size={15} />
							<span class="truncate">All projects</span>
						</button>
						{#each projects as project (project.source + project.id)}
							<button
								class="flex w-full items-center gap-2 rounded-md px-2 py-2 text-left text-xs transition-colors hover:bg-sidebar-accent {selectedProjectId ===
								project.id
									? 'bg-sidebar-accent font-medium'
									: 'text-muted-foreground'}"
								onclick={() => (selectedProjectId = project.id)}
							>
								<FolderSimple size={15} />
								<span class="truncate">{projectLabel(project)}</span>
							</button>
						{/each}
						{#if loadState === 'loading' && projects.length === 0}
							<div class="flex items-center gap-2 px-2 py-3 text-xs text-muted-foreground">
								<CircleNotch class="animate-spin" size={14} /> Loading projects
							</div>
						{:else if loadState === 'ready' && projects.length === 0}
							<p class="px-2 py-3 text-xs text-muted-foreground">No projects imported yet.</p>
						{/if}
					</div>
				</ScrollArea>
			</div>
			<div class="hidden border-t p-4 md:block">
				<p class="text-[11px] font-medium text-muted-foreground">LOCAL INSTANCE</p>
				<p class="mt-1 text-xs text-muted-foreground">127.0.0.1:4117</p>
			</div>
		</aside>

		<main class="min-w-0 flex-1">
			<header
				class="flex min-h-14 flex-wrap items-center justify-between gap-3 border-b px-5 py-3 md:px-8"
			>
				<div>
					<p class="text-xs text-muted-foreground">Overview</p>
					<h1 class="text-base font-semibold tracking-tight">
						{selectedProject ? projectLabel(selectedProject) : 'Usage overview'}
					</h1>
				</div>
				<div class="flex items-center gap-2">
					{#if lastUpdated}<span class="hidden text-[11px] text-muted-foreground sm:inline"
							>Updated {lastUpdated.toLocaleTimeString([], {
								hour: '2-digit',
								minute: '2-digit'
							})}</span
						>{/if}
					<Button
						variant="outline"
						size="sm"
						onclick={loadDashboard}
						disabled={loadState === 'loading'}
					>
						<ArrowsClockwise class={loadState === 'loading' ? 'animate-spin' : ''} /> Refresh
					</Button>
				</div>
			</header>

			<div class="mx-auto max-w-7xl space-y-6 p-5 md:p-8">
				{#if loadState === 'error'}
					<div
						class="flex items-start gap-3 rounded-lg border border-destructive/30 bg-destructive/5 p-4 text-sm"
					>
						<WarningCircle class="mt-0.5 shrink-0 text-destructive" size={18} />
						<div class="min-w-0">
							<p class="font-medium">Analytics service unavailable</p>
							<p class="mt-1 text-xs text-muted-foreground">
								{errorMessage} Start the Rust server with
								<code class="rounded bg-muted px-1 py-0.5">ocstats serve</code> and refresh.
							</p>
						</div>
					</div>
				{/if}

				<section class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
					{#each [{ label: 'Total cost', value: formatCost(totals.cost), detail: 'Across selected sessions', icon: Coins }, { label: 'Total tokens', value: formatNumber(totals.total_tokens), detail: 'Input, output and reasoning', icon: Hash }, { label: 'Sessions', value: formatNumber(visibleSessions.length), detail: 'Recorded usage sessions', icon: Lightning }, { label: 'Models', value: formatNumber(activeModels.length), detail: 'Available in this view', icon: TrendUp }] as metric (metric.label)}
						<Card size="sm"
							><CardContent class="flex items-start justify-between p-4">
								<div>
									<p class="text-xs text-muted-foreground">{metric.label}</p>
									<p class="mt-2 text-2xl font-semibold tracking-tight">{metric.value}</p>
									<p class="mt-1 text-[11px] text-muted-foreground">{metric.detail}</p>
								</div>
								<metric.icon size={17} class="text-muted-foreground" />
							</CardContent></Card
						>
					{/each}
				</section>

				<section class="grid gap-6 xl:grid-cols-[minmax(0,1.35fr)_minmax(280px,0.65fr)]">
					<Card>
						<CardHeader class="flex flex-row items-center justify-between space-y-0"
							><div>
								<CardTitle>Session usage</CardTitle>
								<p class="mt-1 text-xs text-muted-foreground">Usage totals reported by OpenCode</p>
							</div>
							<Badge variant="outline">{visibleSessions.length} sessions</Badge></CardHeader
						>
						<CardContent class="p-0">
							<Table>
								<TableHeader
									><TableRow
										><TableHead class="pl-4">Session</TableHead><TableHead>Source</TableHead
										><TableHead>Tokens</TableHead><TableHead class="pr-4 text-right">Cost</TableHead
										></TableRow
									></TableHeader
								>
								<TableBody>
									{#each visibleSessions.slice(0, 12) as session (session.source + session.session_id)}
										<TableRow
											><TableCell class="pl-4 font-mono text-[11px]"
												>{shortId(session.session_id)}</TableCell
											><TableCell
												><Badge variant="secondary">{session.source_kind}</Badge></TableCell
											><TableCell>{formatNumber(session.usage.total_tokens)}</TableCell><TableCell
												class="pr-4 text-right font-medium"
												>{formatCost(session.usage.cost)}</TableCell
											></TableRow
										>
									{/each}
									{#if loadState === 'loading'}<TableRow
											><TableCell colspan={4} class="h-24 text-center text-muted-foreground"
												><CircleNotch class="mx-auto mb-2 animate-spin" size={16} />Loading usage...</TableCell
											></TableRow
										>{:else if loadState === 'ready' && visibleSessions.length === 0}<TableRow
											><TableCell colspan={4} class="h-24 text-center text-muted-foreground"
												>No usage data for this project.</TableCell
											></TableRow
										>{/if}
								</TableBody>
							</Table>
						</CardContent>
					</Card>

					<Card
						><CardHeader
							><CardTitle>Token mix</CardTitle>
							<p class="mt-1 text-xs text-muted-foreground">
								Distribution across selected sessions
							</p></CardHeader
						><CardContent class="space-y-4">
							{#each [{ label: 'Input', value: totals.input_tokens, color: 'bg-foreground' }, { label: 'Output', value: totals.output_tokens, color: 'bg-muted-foreground' }, { label: 'Reasoning', value: totals.reasoning_tokens, color: 'bg-border' }] as item (item.label)}
								<div>
									<div class="mb-1.5 flex justify-between text-xs">
										<span>{item.label}</span><span class="text-muted-foreground"
											>{formatNumber(item.value)}</span
										>
									</div>
									<div class="h-1.5 overflow-hidden rounded-full bg-muted">
										<div
											class="h-full rounded-full {item.color}"
											style={`width: ${totals.total_tokens ? Math.min(100, (item.value / totals.total_tokens) * 100) : 0}%`}
										></div>
									</div>
								</div>
							{/each}
							<Separator />
							<div class="flex items-center justify-between text-xs">
								<span class="text-muted-foreground">Cache read</span><span class="font-medium"
									>{formatNumber(totals.cache_read_tokens)}</span
								>
							</div>
							<div class="flex items-center justify-between text-xs">
								<span class="text-muted-foreground">Cache write</span><span class="font-medium"
									>{formatNumber(totals.cache_write_tokens)}</span
								>
							</div>
						</CardContent></Card
					>
				</section>
			</div>
		</main>
	</div>
</div>
