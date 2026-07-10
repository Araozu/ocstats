<script lang="ts">
	import ArrowsClockwiseIcon from 'phosphor-svelte/lib/ArrowsClockwiseIcon';
	import ArrowUpRightIcon from 'phosphor-svelte/lib/ArrowUpRightIcon';
	import ChartLineUpIcon from 'phosphor-svelte/lib/ChartLineUpIcon';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import CircleNotchIcon from 'phosphor-svelte/lib/CircleNotchIcon';
	import CoinsIcon from 'phosphor-svelte/lib/CoinsIcon';
	import DatabaseIcon from 'phosphor-svelte/lib/DatabaseIcon';
	import FolderSimpleIcon from 'phosphor-svelte/lib/FolderSimpleIcon';
	import HashIcon from 'phosphor-svelte/lib/HashIcon';
	import LightningIcon from 'phosphor-svelte/lib/LightningIcon';
	import MonitorIcon from 'phosphor-svelte/lib/MonitorIcon';
	import MoonIcon from 'phosphor-svelte/lib/MoonIcon';
	import SunIcon from 'phosphor-svelte/lib/SunIcon';
	import TrendUpIcon from 'phosphor-svelte/lib/TrendUpIcon';
	import WarningCircleIcon from 'phosphor-svelte/lib/WarningCircleIcon';
	import { resetMode, setMode } from 'mode-watcher';
	import { Badge } from '$lib/components/ui/badge';
	import { Button, buttonVariants } from '$lib/components/ui/button';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
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
	type SessionUsage = {
		source: string;
		session_id: string;
		project_id: string;
		title: string;
		usage: Usage;
		source_kind: string;
	};
	type ModelUsage = { provider_id: string; model_id: string; variant: string | null; usage: Usage };
	type SessionDetail = SessionUsage & { models: ModelUsage[] };
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
	let selectedProjectKey = $state('all');
	let selectedSessionId = $state<string | null>(null);
	let selectedSessionDetail = $state<SessionDetail | null>(null);
	let sessionDetailLoading = $state(false);
	let loadState = $state<LoadState>('loading');
	let errorMessage = $state('');
	let lastUpdated = $state<Date | null>(null);

	const projectKey = (project: Project) => `${project.source}:${project.id}`;
	const sessionKey = (session: SessionUsage) => `${session.source}:${session.session_id}`;
	const selectedProject = $derived(
		projects.find((project) => projectKey(project) === selectedProjectKey) ?? null
	);
	const visibleSessions = $derived(
		selectedProject
			? sessions.filter(
					(session) =>
						session.project_id === selectedProject.id && session.source === selectedProject.source
				)
			: sessions
	);
	const selectedSession = $derived(
		visibleSessions.find((session) => sessionKey(session) === selectedSessionId) ?? null
	);
	const totals = $derived(
		visibleSessions.reduce((total, session) => addUsage(total, session.usage), emptyUsage)
	);
	const totalBilledTokens = $derived(
		totals.input_tokens + totals.output_tokens + totals.reasoning_tokens
	);
	const projectName = $derived(selectedProject ? projectLabel(selectedProject) : 'All projects');

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

	function selectProject(key: string) {
		selectedProjectKey = key;
		selectedSessionId = null;
		selectedSessionDetail = null;
	}

	async function selectSession(session: SessionUsage) {
		selectedSessionId = sessionKey(session);
		selectedSessionDetail = null;
		sessionDetailLoading = true;
		try {
			const params = new URLSearchParams({
				source: session.source,
				session_id: session.session_id
			});
			const response = await fetch(`${API_URL}/usage/session?${params}`);
			if (!response.ok) throw new Error('Could not load session details.');
			selectedSessionDetail = await response.json();
		} catch (error) {
			errorMessage = error instanceof Error ? error.message : 'Could not load session details.';
		} finally {
			sessionDetailLoading = false;
		}
	}

	function clearSession() {
		selectedSessionId = null;
		selectedSessionDetail = null;
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

	function percent(value: number) {
		return totalBilledTokens ? Math.min(100, (value / totalBilledTokens) * 100) : 0;
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
	<div class="grid min-h-screen lg:grid-cols-[17rem_minmax(0,1fr)]">
		<aside class="border-b bg-sidebar lg:border-r lg:border-b-0">
			<div class="flex h-16 items-center gap-3 border-b px-5">
				<div
					class="flex size-8 items-center justify-center rounded-lg bg-primary text-primary-foreground"
				>
					<ChartLineUpIcon size={18} weight="bold" />
				</div>
				<div>
					<p class="text-sm font-semibold tracking-tight">ocstats</p>
					<p class="text-[11px] text-muted-foreground">OpenCode usage intelligence</p>
				</div>
			</div>

			<div
				class="grid gap-5 p-4 lg:sticky lg:top-0 lg:max-h-screen lg:grid-rows-[auto_minmax(0,1fr)_auto]"
			>
				<section>
					<div class="mb-2 flex items-center justify-between px-2">
						<p class="text-[11px] font-semibold uppercase tracking-[0.12em] text-muted-foreground">
							Projects
						</p>
						<Badge variant="secondary">{projects.length}</Badge>
					</div>
					<ScrollArea class="max-h-48 lg:max-h-[30vh]">
						<div class="space-y-1 pr-3">
							<button
								class="flex w-full items-center gap-2 rounded-md px-2.5 py-2 text-left text-xs transition-colors hover:bg-sidebar-accent {selectedProjectKey ===
								'all'
									? 'bg-sidebar-accent font-medium text-sidebar-accent-foreground'
									: 'text-muted-foreground'}"
								onclick={() => selectProject('all')}
							>
								<DatabaseIcon size={15} />
								<span class="truncate">All projects</span>
							</button>
							{#each projects as project (projectKey(project))}
								<button
									class="flex w-full items-center gap-2 rounded-md px-2.5 py-2 text-left text-xs transition-colors hover:bg-sidebar-accent {selectedProjectKey ===
									projectKey(project)
										? 'bg-sidebar-accent font-medium text-sidebar-accent-foreground'
										: 'text-muted-foreground'}"
									onclick={() => selectProject(projectKey(project))}
								>
									<FolderSimpleIcon size={15} />
									<span class="truncate">{projectLabel(project)}</span>
								</button>
							{/each}
							{#if loadState === 'loading' && projects.length === 0}
								<div class="flex items-center gap-2 px-2 py-3 text-xs text-muted-foreground">
									<CircleNotchIcon class="animate-spin" size={14} /> Loading projects
								</div>
							{:else if loadState === 'ready' && projects.length === 0}
								<p class="px-2 py-3 text-xs text-muted-foreground">No projects imported yet.</p>
							{/if}
						</div>
					</ScrollArea>
				</section>

				<section class="min-h-0">
					<div class="mb-2 flex items-center justify-between px-2">
						<p class="text-[11px] font-semibold uppercase tracking-[0.12em] text-muted-foreground">
							Sessions
						</p>
						<Badge variant="secondary">{visibleSessions.length}</Badge>
					</div>
					<ScrollArea class="max-h-64 lg:h-full lg:max-h-none">
						<div class="space-y-1 pr-3">
							{#each visibleSessions as session (sessionKey(session))}
								<button
									class="w-full rounded-md border border-transparent px-2.5 py-2.5 text-left transition-colors hover:bg-sidebar-accent {selectedSessionId ===
									sessionKey(session)
										? 'border-sidebar-border bg-sidebar-accent'
										: ''}"
									onclick={() => void selectSession(session)}
								>
									<p class="truncate text-xs font-medium">{session.title || 'Untitled session'}</p>
									<div
										class="mt-1 flex items-center justify-between gap-2 text-[10px] text-muted-foreground"
									>
										<span class="truncate font-mono">{shortId(session.session_id)}</span>
										<span class="shrink-0">{formatCost(session.usage.cost)}</span>
									</div>
								</button>
							{:else}
								{#if loadState === 'loading'}
									<div class="flex items-center gap-2 px-2 py-3 text-xs text-muted-foreground">
										<CircleNotchIcon class="animate-spin" size={14} /> Loading sessions
									</div>
								{:else}
									<p class="px-2 py-3 text-xs text-muted-foreground">No sessions in this view.</p>
								{/if}
							{/each}
						</div>
					</ScrollArea>
				</section>

				<div
					class="hidden items-center gap-2 border-t pt-4 text-[11px] text-muted-foreground lg:flex"
				>
					<CheckCircleIcon size={14} class="text-primary" /> Local service · 127.0.0.1:4117
				</div>
			</div>
		</aside>

		<main class="min-w-0">
			<header
				class="flex min-h-16 flex-wrap items-center justify-between gap-3 border-b px-5 py-3 md:px-8"
			>
				<div>
					<p class="text-xs text-muted-foreground">
						{selectedSession ? 'Session details' : 'Usage overview'}
					</p>
					<h1 class="mt-0.5 text-lg font-semibold tracking-tight">
						{selectedSession?.title || projectName}
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
						><ArrowsClockwiseIcon class={loadState === 'loading' ? 'animate-spin' : ''} /> Refresh</Button
					>
					<DropdownMenu.Root>
						<DropdownMenu.Trigger
							class={buttonVariants({ variant: 'outline', size: 'icon' })}
							aria-label="Change color theme"
							><SunIcon class="dark:hidden" /><MoonIcon class="hidden dark:block" /><span
								class="sr-only">Change color theme</span
							></DropdownMenu.Trigger
						>
						<DropdownMenu.Content align="end">
							<DropdownMenu.Item onclick={() => setMode('light')}
								><SunIcon /> Light</DropdownMenu.Item
							>
							<DropdownMenu.Item onclick={() => setMode('dark')}
								><MoonIcon /> Dark</DropdownMenu.Item
							>
							<DropdownMenu.Item onclick={resetMode}><MonitorIcon /> System</DropdownMenu.Item>
						</DropdownMenu.Content>
					</DropdownMenu.Root>
				</div>
			</header>

			<div class="mx-auto max-w-7xl space-y-7 p-5 md:p-8">
				{#if loadState === 'error'}
					<div
						class="flex items-start gap-3 rounded-lg border border-destructive/30 bg-destructive/5 p-4 text-sm"
					>
						<WarningCircleIcon class="mt-0.5 shrink-0 text-destructive" size={18} />
						<div>
							<p class="font-medium">Analytics service unavailable</p>
							<p class="mt-1 text-xs text-muted-foreground">
								{errorMessage} Start the Rust server with
								<code class="rounded bg-muted px-1 py-0.5">ocstats serve</code> and refresh.
							</p>
						</div>
					</div>
				{/if}

				{#if sessionDetailLoading}
					<div
						class="flex min-h-64 items-center justify-center gap-2 text-sm text-muted-foreground"
					>
						<CircleNotchIcon class="animate-spin" size={17} /> Loading session details...
					</div>
				{:else if selectedSessionDetail}
					<div class="space-y-7">
						<div class="flex flex-wrap items-start justify-between gap-4">
							<div>
								<Button variant="ghost" size="sm" class="-ml-2 mb-3" onclick={clearSession}
									>← Back to overview</Button
								>
								<p class="text-xs font-medium uppercase tracking-[0.12em] text-muted-foreground">
									Selected session
								</p>
								<h2 class="mt-2 text-2xl font-semibold tracking-tight">
									{selectedSessionDetail.title || 'Untitled session'}
								</h2>
								<p class="mt-2 font-mono text-xs text-muted-foreground">
									{selectedSessionDetail.source} · {shortId(selectedSessionDetail.session_id)}
								</p>
							</div>
							<Badge variant="secondary">{selectedSessionDetail.source_kind}</Badge>
						</div>
						<section class="grid gap-3 sm:grid-cols-2 xl:grid-cols-5">
							{#each [{ label: 'Input tokens', value: selectedSessionDetail.usage.input_tokens }, { label: 'Cached tokens', value: selectedSessionDetail.usage.cache_read_tokens }, { label: 'Reasoning', value: selectedSessionDetail.usage.reasoning_tokens }, { label: 'Output tokens', value: selectedSessionDetail.usage.output_tokens }, { label: 'Total cost', value: formatCost(selectedSessionDetail.usage.cost) }] as metric (metric.label)}
								<Card size="sm"
									><CardContent class="p-4"
										><p class="text-xs text-muted-foreground">{metric.label}</p>
										<p class="mt-2 text-2xl font-semibold tracking-tight">
											{typeof metric.value === 'string' ? metric.value : formatNumber(metric.value)}
										</p></CardContent
									></Card
								>
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
											><TableHead class="pl-5">Model</TableHead><TableHead>Input</TableHead
											><TableHead>Cached</TableHead><TableHead>Reasoning</TableHead><TableHead
												class="pr-5">Output</TableHead
											></TableRow
										></TableHeader
									><TableBody
										>{#each selectedSessionDetail.models as model (model.provider_id + model.model_id + model.variant)}<TableRow
												><TableCell class="pl-5"
													><p class="text-xs font-medium">{model.model_id}</p>
													<p class="text-[11px] text-muted-foreground">
														{model.provider_id}{model.variant ? ` · ${model.variant}` : ''}
													</p></TableCell
												><TableCell>{formatNumber(model.usage.input_tokens)}</TableCell><TableCell
													>{formatNumber(model.usage.cache_read_tokens)}</TableCell
												><TableCell>{formatNumber(model.usage.reasoning_tokens)}</TableCell
												><TableCell class="pr-5"
													>{formatNumber(model.usage.output_tokens)}</TableCell
												></TableRow
											>{:else}<TableRow
												><TableCell colspan={5} class="h-24 text-center text-muted-foreground"
													>No model usage records.</TableCell
												></TableRow
											>{/each}</TableBody
									></Table
								></CardContent
							></Card
						>
					</div>
				{:else}
					<section class="flex flex-wrap items-end justify-between gap-4">
						<div>
							<p class="text-xs font-medium uppercase tracking-[0.12em] text-muted-foreground">
								Scope
							</p>
							<h2 class="mt-1 text-2xl font-semibold tracking-tight">{projectName}</h2>
							<p class="mt-2 text-sm text-muted-foreground">
								A concise view of your recorded OpenCode usage.
							</p>
						</div>
						<Badge variant="outline">{visibleSessions.length} sessions</Badge>
					</section>
					<section class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
						{#each [{ label: 'Total cost', value: formatCost(totals.cost), detail: 'Across this selection', icon: CoinsIcon }, { label: 'Total tokens', value: formatNumber(totals.total_tokens), detail: 'All reported tokens', icon: HashIcon }, { label: 'Sessions', value: formatNumber(visibleSessions.length), detail: 'Recorded conversations', icon: LightningIcon }, { label: 'Models', value: formatNumber(models.length), detail: 'Available models', icon: TrendUpIcon }] as metric (metric.label)}
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
											><TableHead class="pl-5">Session</TableHead><TableHead>Source</TableHead
											><TableHead>Tokens</TableHead><TableHead class="pr-5 text-right"
												>Cost</TableHead
											></TableRow
										></TableHeader
									><TableBody
										>{#each visibleSessions.slice(0, 12) as session (sessionKey(session))}<TableRow
												class="cursor-pointer"
												onclick={() => void selectSession(session)}
												><TableCell class="pl-5"
													><p class="max-w-64 truncate text-xs font-medium">
														{session.title || 'Untitled session'}
													</p>
													<p class="mt-0.5 font-mono text-[10px] text-muted-foreground">
														{shortId(session.session_id)}
													</p></TableCell
												><TableCell
													><Badge variant="secondary">{session.source_kind}</Badge></TableCell
												><TableCell>{formatNumber(session.usage.total_tokens)}</TableCell><TableCell
													class="pr-5 text-right font-medium"
													>{formatCost(session.usage.cost)}</TableCell
												></TableRow
											>{:else}{#if loadState === 'loading'}<TableRow
													><TableCell colspan={4} class="h-28 text-center text-muted-foreground"
														><CircleNotchIcon class="mx-auto mb-2 animate-spin" size={16} />Loading
														usage...</TableCell
													></TableRow
												>{:else}<TableRow
													><TableCell colspan={4} class="h-28 text-center text-muted-foreground"
														>No usage data for this project.</TableCell
													></TableRow
												>{/if}{/each}</TableBody
									></Table
								></CardContent
							></Card
						>
						<Card
							><CardHeader
								><CardTitle>Token mix</CardTitle>
								<p class="mt-1 text-xs text-muted-foreground">
									Billable usage composition.
								</p></CardHeader
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
				{/if}
			</div>
		</main>
	</div>
</div>
