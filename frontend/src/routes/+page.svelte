<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { createQuery } from '@tanstack/svelte-query';
	import type { ModelPricing, SessionUsage } from '$lib/api/ocstats';
	import {
		addUsage,
		emptyUsage,
		projectKey,
		projectLabel,
		sessionKey
	} from '$lib/components/dashboard/format';
	import Overview from '$lib/components/dashboard/overview.svelte';
	import SessionDetail from '$lib/components/dashboard/session-detail.svelte';
	import ProjectSidebar from '$lib/components/dashboard/project-sidebar.svelte';
	import SessionSidebar from '$lib/components/dashboard/session-sidebar.svelte';
	import { usageQueries } from '$lib/queries/usage';
	import CircleNotchIcon from 'phosphor-svelte/lib/CircleNotchIcon';
	import WarningCircleIcon from 'phosphor-svelte/lib/WarningCircleIcon';

	const projectsQuery = createQuery(usageQueries.projects);
	const sessionsQuery = createQuery(usageQueries.sessions);
	const modelsQuery = createQuery(usageQueries.models);
	const pricingQuery = createQuery(usageQueries.pricing);

	const projects = $derived(projectsQuery.data ?? []);
	const sessions = $derived(sessionsQuery.data ?? []);
	const models = $derived(modelsQuery.data ?? []);
	let pricing = $state<ModelPricing[]>([]);
	$effect(() => {
		pricing = pricingQuery.data?.models ?? [];
	});
	const selectedProjectKey = $derived(page.url.searchParams.get('project') ?? 'all');
	const selectedSession = $derived.by(() => {
		const source = page.url.searchParams.get('source');
		const sessionId = page.url.searchParams.get('session_id');
		return source && sessionId
			? (sessions.find(
					(session) => session.source === source && session.session_id === sessionId
				) ?? null)
			: null;
	});
	const sessionQuery = createQuery(() =>
		usageQueries.session(selectedSession?.source ?? null, selectedSession?.session_id ?? null)
	);
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
	const totals = $derived(
		visibleSessions.reduce((total, session) => addUsage(total, session.usage), emptyUsage)
	);
	const projectName = $derived(selectedProject ? projectLabel(selectedProject) : 'All projects');
	const lastUpdated = $derived(
		sessionsQuery.dataUpdatedAt ? new Date(sessionsQuery.dataUpdatedAt) : null
	);
	const error = $derived(
		projectsQuery.error ??
			sessionsQuery.error ??
			modelsQuery.error ??
			pricingQuery.error ??
			sessionQuery.error
	);
	const isRefreshing = $derived(
		projectsQuery.isFetching ||
			sessionsQuery.isFetching ||
			modelsQuery.isFetching ||
			pricingQuery.isFetching
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

	function refreshDashboard() {
		void Promise.all([
			projectsQuery.refetch(),
			sessionsQuery.refetch(),
			modelsQuery.refetch(),
			pricingQuery.refetch()
		]);
	}
</script>

<svelte:head>
	<title>Usage overview | ocstats</title>
	<meta name="description" content="OpenCode usage and token statistics" />
</svelte:head>

<div class="min-h-screen bg-background text-foreground">
	<div class="grid min-h-screen lg:grid-cols-[16rem_19rem_minmax(0,1fr)]">
		<ProjectSidebar
			{projects}
			{selectedProjectKey}
			{lastUpdated}
			{isRefreshing}
			onRefresh={refreshDashboard}
			onSelect={selectProject}
		/>
		<SessionSidebar
			sessions={visibleSessions}
			{projectName}
			selectedSessionKey={selectedSession
				? sessionKey(selectedSession.source, selectedSession.session_id)
				: null}
			isLoading={sessionsQuery.isPending}
			onSelect={selectSession}
		/>
		<main class="min-w-0">
			<div class="mx-auto max-w-7xl space-y-7 p-5 md:p-8">
				{#if error}
					<div
						class="flex items-start gap-3 rounded-lg border border-destructive/30 bg-destructive/5 p-4 text-sm"
					>
						<WarningCircleIcon class="mt-0.5 shrink-0 text-destructive" size={18} />
						<div>
							<p class="font-medium">Analytics service unavailable</p>
							<p class="mt-1 text-xs text-muted-foreground">
								{error.message} Start the Rust server with
								<code class="rounded bg-muted px-1 py-0.5">ocstats serve</code> and refresh.
							</p>
						</div>
					</div>
				{/if}
				{#if selectedSession && sessionQuery.isPending}
					<div
						class="flex min-h-64 items-center justify-center gap-2 text-sm text-muted-foreground"
					>
						<CircleNotchIcon class="animate-spin" size={17} /> Loading session details...
					</div>
				{:else if sessionQuery.data}
					<SessionDetail
						session={sessionQuery.data}
						{pricing}
						onBack={() => updateSelection({ source: null, session_id: null })}
					/>
				{:else}
					<Overview
						{projectName}
						sessions={visibleSessions}
						modelCount={models.length}
						{totals}
						isLoading={sessionsQuery.isPending}
						onSessionSelect={selectSession}
					/>
				{/if}
			</div>
		</main>
	</div>
</div>
