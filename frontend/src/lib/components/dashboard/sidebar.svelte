<script lang="ts">
	import ChartLineUpIcon from 'phosphor-svelte/lib/ChartLineUpIcon';
	import CircleNotchIcon from 'phosphor-svelte/lib/CircleNotchIcon';
	import DatabaseIcon from 'phosphor-svelte/lib/DatabaseIcon';
	import FolderSimpleIcon from 'phosphor-svelte/lib/FolderSimpleIcon';
	import type { Project, SessionUsage } from '$lib/api/ocstats';
	import { Badge } from '$lib/components/ui/badge';
	import { formatCost, projectKey, projectLabel, sessionKey, shortId } from './format';

	let {
		projects,
		sessions,
		selectedProjectKey,
		selectedSessionKey,
		isLoading = false,
		onProjectSelect,
		onSessionSelect
	}: {
		projects: Project[];
		sessions: SessionUsage[];
		selectedProjectKey: string;
		selectedSessionKey: string | null;
		isLoading?: boolean;
		onProjectSelect: (key: string) => void;
		onSessionSelect: (session: SessionUsage) => void;
	} = $props();
</script>

<aside class="border-b bg-sidebar lg:h-screen lg:border-r lg:border-b-0">
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

	<div class="grid gap-5 p-4 lg:h-[calc(100vh-4rem)] lg:grid-rows-[auto_minmax(0,1fr)]">
		<section>
			<div class="mb-2 flex items-center justify-between px-2">
				<p class="text-[11px] font-semibold uppercase tracking-[0.12em] text-muted-foreground">
					Projects
				</p>
				<Badge variant="secondary">{projects.length}</Badge>
			</div>
			<div class="flex gap-1 overflow-x-auto pb-1 lg:max-h-44 lg:flex-col lg:overflow-y-auto">
				<button
					class="flex shrink-0 items-center gap-2 rounded-md px-2.5 py-2 text-left text-xs transition-colors hover:bg-sidebar-accent {selectedProjectKey ===
					'all'
						? 'bg-sidebar-accent font-medium text-sidebar-accent-foreground'
						: 'text-muted-foreground'}"
					onclick={() => onProjectSelect('all')}
				>
					<DatabaseIcon size={15} /><span class="whitespace-nowrap">All projects</span>
				</button>
				{#each projects as project (projectKey(project))}
					<button
						class="flex shrink-0 items-center gap-2 rounded-md px-2.5 py-2 text-left text-xs transition-colors hover:bg-sidebar-accent {selectedProjectKey ===
						projectKey(project)
							? 'bg-sidebar-accent font-medium text-sidebar-accent-foreground'
							: 'text-muted-foreground'}"
						onclick={() => onProjectSelect(projectKey(project))}
					>
						<FolderSimpleIcon size={15} /><span class="max-w-44 truncate"
							>{projectLabel(project)}</span
						>
					</button>
				{/each}
			</div>
		</section>

		<section class="min-h-0">
			<div class="mb-2 flex items-center justify-between px-2">
				<p class="text-[11px] font-semibold uppercase tracking-[0.12em] text-muted-foreground">
					Sessions
				</p>
				<Badge variant="secondary">{sessions.length}</Badge>
			</div>
			<div class="grid max-h-64 gap-1 overflow-y-auto pr-1 lg:max-h-none lg:grid-cols-1">
				{#each sessions as session (sessionKey(session.source, session.session_id))}
					<button
						class="w-full rounded-md border border-transparent px-2.5 py-2.5 text-left transition-colors hover:bg-sidebar-accent {selectedSessionKey ===
						sessionKey(session.source, session.session_id)
							? 'border-sidebar-border bg-sidebar-accent'
							: ''}"
						onclick={() => onSessionSelect(session)}
					>
						<p class="truncate text-xs font-medium">{session.title || 'Untitled session'}</p>
						<div
							class="mt-1 flex items-center justify-between gap-2 text-[10px] text-muted-foreground"
						>
							<span class="truncate font-mono">{shortId(session.session_id)}</span><span
								class="shrink-0">{formatCost(session.usage.cost)}</span
							>
						</div>
					</button>
				{:else}
					{#if isLoading}
						<div class="flex items-center gap-2 px-2 py-3 text-xs text-muted-foreground">
							<CircleNotchIcon class="animate-spin" size={14} /> Loading sessions
						</div>
					{:else}
						<p class="px-2 py-3 text-xs text-muted-foreground">No sessions in this view.</p>
					{/if}
				{/each}
			</div>
		</section>
	</div>
</aside>
