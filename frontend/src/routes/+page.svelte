<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { SvelteSet } from 'svelte/reactivity';
	import { getAuthStatus, importData, login, type SessionUsage } from '$lib/api/ocstats';
	import {
		addUsage,
		emptyUsage,
		projectKey,
		projectLabel,
		sessionKey,
		sortSessionsByDate
	} from '$lib/components/dashboard/format';
	import Overview from '$lib/components/dashboard/overview.svelte';
	import DashboardSkeleton from '$lib/components/dashboard/dashboard-skeleton.svelte';
	import SessionDetail from '$lib/components/dashboard/session-detail.svelte';
	import ProjectSidebar from '$lib/components/dashboard/project-sidebar.svelte';
	import SessionSidebar from '$lib/components/dashboard/session-sidebar.svelte';
	import MobileNavigation from '$lib/components/dashboard/mobile-navigation.svelte';
	import { setModelPricingContext } from '$lib/model-pricing';
	import { usageQueries } from '$lib/queries/usage';
	import WarningCircleIcon from 'phosphor-svelte/lib/WarningCircleIcon';

	const authQuery = createQuery(() => ({ queryKey: ['auth'], queryFn: getAuthStatus }));
	const queryClient = useQueryClient();
	const authenticated = $derived(authQuery.data?.authenticated === true);
	const projectsQuery = createQuery(() => usageQueries.projects(authenticated));
	const sessionsQuery = createQuery(() => usageQueries.sessions(authenticated));
	const modelsQuery = createQuery(() => usageQueries.models(authenticated));
	const pricingQuery = createQuery(() => usageQueries.pricing(authenticated));

	const projects = $derived(projectsQuery.data ?? []);
	const sessions = $derived(sessionsQuery.data ?? []);
	const models = $derived(modelsQuery.data ?? []);
	const pricing = $derived(pricingQuery.data?.models ?? []);
	const modelPricing = setModelPricingContext();
	$effect(() => modelPricing.set(pricing, pricingQuery.data !== undefined));
	let projectsCollapsed = $state(false);
	let sessionSortDirection = $state<'asc' | 'desc'>('desc');
	let previousProjectKey = $state<string | undefined>(undefined);
	let expandedSessionKeys = new SvelteSet<string>();
	let revealedSessionKey = $state<string | null>(null);
	const selectedProjectKey = $derived(page.url.searchParams.get('project') ?? 'all');
	$effect(() => {
		if (previousProjectKey !== undefined && previousProjectKey !== selectedProjectKey) {
			expandedSessionKeys.clear();
			revealedSessionKey = null;
		}
		previousProjectKey = selectedProjectKey;
	});
	const selectedSession = $derived.by(() => {
		const source = page.url.searchParams.get('source');
		const sessionId = page.url.searchParams.get('session_id');
		return source && sessionId
			? (sessions.find(
					(session) => session.source === source && session.session_id === sessionId
				) ?? null)
			: null;
	});
	const selectedSessionKey = $derived(
		selectedSession ? sessionKey(selectedSession.source, selectedSession.session_id) : null
	);
	$effect(() => {
		if (selectedSessionKey === null) revealedSessionKey = null;
	});
	const sessionQuery = createQuery(() =>
		usageQueries.session(
			selectedSession?.source ?? null,
			selectedSession?.session_id ?? null,
			authenticated
		)
	);
	const selectedProject = $derived(
		projects.find((project) => projectKey(project) === selectedProjectKey) ?? null
	);
	const modelUsageQuery = createQuery(() =>
		usageQueries.modelUsage(selectedProject?.id ?? null, authenticated)
	);
	let password = $state('');
	let loginError = $state<string | null>(null);
	let isLoggingIn = $state(false);
	let importError = $state<Error | null>(null);
	let isImporting = $state(false);
	const visibleSessions = $derived.by(() => {
		const filtered = selectedProject
			? sessions.filter(
					(session) =>
						session.project_id === selectedProject.id && session.source === selectedProject.source
				)
			: sessions;
		return sortSessionsByDate(filtered, sessionSortDirection);
	});
	const totals = $derived(
		visibleSessions.reduce((total, session) => addUsage(total, session.usage), emptyUsage)
	);
	const projectName = $derived(selectedProject ? projectLabel(selectedProject) : 'All projects');
	const lastUpdated = $derived(
		sessionsQuery.dataUpdatedAt ? new Date(sessionsQuery.dataUpdatedAt) : null
	);
	const error = $derived(
		importError ??
			projectsQuery.error ??
			sessionsQuery.error ??
			modelsQuery.error ??
			pricingQuery.error ??
			modelUsageQuery.error ??
			sessionQuery.error
	);
	function updateSelection(updates: Record<string, string | null>) {
		const url = new URL(page.url);
		for (const [key, value] of Object.entries(updates)) {
			if (value) url.searchParams.set(key, value);
			else url.searchParams.delete(key);
		}
		void goto(url, { replaceState: true, noScroll: true, keepFocus: true });
	}

	function selectProject(key: string) {
		updateSelection({ project: key, source: null, session_id: null });
	}

	function selectSession(session: SessionUsage) {
		updateSelection({
			project: `${session.source}:${session.project_id}`,
			source: session.source,
			session_id: session.session_id
		});
	}

	function toggleSessionSort() {
		sessionSortDirection = sessionSortDirection === 'desc' ? 'asc' : 'desc';
	}

	async function importDashboard() {
		importError = null;
		isImporting = true;
		try {
			await importData();
			await queryClient.invalidateQueries({ refetchType: 'all' });
		} catch (error) {
			importError = error instanceof Error ? error : new Error('Unable to import OpenCode data.');
		} finally {
			isImporting = false;
		}
	}

	async function submitLogin() {
		loginError = null;
		isLoggingIn = true;
		try {
			await login(password);
			password = '';
			await authQuery.refetch();
		} catch (error) {
			loginError = error instanceof Error ? error.message : 'Unable to sign in.';
		} finally {
			isLoggingIn = false;
		}
	}
</script>

<svelte:head>
	<title>Usage overview | ocstats</title>
	<meta name="description" content="OpenCode usage and token statistics" />
</svelte:head>

<div class="min-h-screen bg-background text-foreground">
	{#if authQuery.isPending}
		<div class="grid min-h-screen place-items-center text-sm text-muted-foreground">
			Checking access...
		</div>
	{:else if !authenticated}
		<main class="grid min-h-screen place-items-center px-5">
			<form
				class="w-full max-w-sm space-y-5 rounded-xl border bg-card p-6 shadow-sm"
				onsubmit={(event) => {
					event.preventDefault();
					void submitLogin();
				}}
			>
				<div class="space-y-1.5">
					<p class="text-xs font-semibold uppercase tracking-[0.2em] text-primary">ocstats</p>
					<h1 class="text-xl font-semibold">Sign in to usage analytics</h1>
					<p class="text-sm text-muted-foreground">Enter the dashboard password to continue.</p>
				</div>
				<label class="grid gap-2 text-sm font-medium">
					Password
					<input
						class="h-10 rounded-md border bg-background px-3 text-sm"
						type="password"
						bind:value={password}
						autocomplete="current-password"
					/>
				</label>
				{#if loginError}<p class="text-sm text-destructive">{loginError}</p>{/if}
				<button
					class="h-10 w-full rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground disabled:opacity-50"
					type="submit"
					disabled={isLoggingIn || !password}
				>
					{isLoggingIn ? 'Signing in...' : 'Sign in'}
				</button>
			</form>
		</main>
	{:else}
		<MobileNavigation
			{projects}
			sessions={visibleSessions}
			{projectName}
			{selectedProjectKey}
			{selectedSessionKey}
			expandedKeys={expandedSessionKeys}
			{revealedSessionKey}
			sortDirection={sessionSortDirection}
			{isImporting}
			isLoading={sessionsQuery.isPending}
			onImport={importDashboard}
			onOverview={() => updateSelection({ source: null, session_id: null })}
			onProjectSelect={selectProject}
			onSessionSelect={selectSession}
			onToggleSort={toggleSessionSort}
			onAncestorsRevealed={(key) => (revealedSessionKey = key)}
		/>
		<div
			class="grid min-h-[calc(100dvh-3.5rem)] min-w-0 xl:min-h-screen {projectsCollapsed
				? 'xl:grid-cols-[3.5rem_19rem_minmax(0,1fr)]'
				: 'xl:grid-cols-[16rem_19rem_minmax(0,1fr)]'}"
		>
			<div class="hidden xl:sticky xl:top-0 xl:block xl:h-dvh xl:self-start">
				<ProjectSidebar
					{projects}
					{selectedProjectKey}
					{lastUpdated}
					{isImporting}
					collapsed={projectsCollapsed}
					onImport={importDashboard}
					onSelect={selectProject}
					onToggle={() => (projectsCollapsed = !projectsCollapsed)}
				/>
			</div>
			<div class="hidden xl:sticky xl:top-0 xl:block xl:h-dvh xl:self-start">
				<SessionSidebar
					sessions={visibleSessions}
					{projectName}
					expandedKeys={expandedSessionKeys}
					{revealedSessionKey}
					sortDirection={sessionSortDirection}
					{selectedSessionKey}
					isLoading={sessionsQuery.isPending}
					onOverview={() => updateSelection({ source: null, session_id: null })}
					onSelect={selectSession}
					onToggleSort={toggleSessionSort}
					onAncestorsRevealed={(key) => (revealedSessionKey = key)}
				/>
			</div>
			<main class="min-w-0">
				<div class="mx-auto max-w-7xl space-y-7 px-4 py-5 sm:px-5 md:px-8">
					{#if error}
						<div
							class="flex items-start gap-3 rounded-lg border border-destructive/30 bg-destructive/5 p-4 text-sm"
						>
							<WarningCircleIcon class="mt-0.5 shrink-0 text-destructive" size={18} />
							<div class="min-w-0">
								<p class="font-medium">Analytics service unavailable</p>
								<p class="mt-1 break-words text-xs text-muted-foreground">
									{error.message} Start the Rust server with
									<code class="rounded bg-muted px-1 py-0.5">ocstats serve</code> and refresh.
								</p>
							</div>
						</div>
					{/if}
					{#if sessionsQuery.isPending || (selectedSession && sessionQuery.isPending)}
						<DashboardSkeleton rows={selectedSession ? 6 : 4} />
					{:else if sessionQuery.data}
						<SessionDetail session={sessionQuery.data} />
					{:else}
						<Overview
							{projectName}
							sessions={visibleSessions}
							modelCount={models.length}
							{totals}
							modelUsage={modelUsageQuery.data ?? []}
							isLoading={sessionsQuery.isPending}
							onSessionSelect={selectSession}
						/>
					{/if}
				</div>
			</main>
		</div>
	{/if}
</div>
