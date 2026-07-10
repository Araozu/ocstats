<script lang="ts">
	import ChartLineUpIcon from 'phosphor-svelte/lib/ChartLineUpIcon';
	import DatabaseIcon from 'phosphor-svelte/lib/DatabaseIcon';
	import FolderSimpleIcon from 'phosphor-svelte/lib/FolderSimpleIcon';
	import type { Project } from '$lib/api/ocstats';
	import { Badge } from '$lib/components/ui/badge';
	import { projectKey, projectLabel } from './format';

	let {
		projects,
		selectedProjectKey,
		onSelect
	}: { projects: Project[]; selectedProjectKey: string; onSelect: (key: string) => void } =
		$props();
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
	<div class="p-4 lg:h-[calc(100vh-4rem)]">
		<div class="mb-2 flex items-center justify-between px-2">
			<p class="text-[11px] font-semibold uppercase tracking-[0.12em] text-muted-foreground">
				Projects
			</p>
			<Badge variant="secondary">{projects.length}</Badge>
		</div>
		<div
			class="flex gap-1 overflow-x-auto pb-1 lg:h-[calc(100%-2rem)] lg:flex-col lg:overflow-y-auto"
		>
			<button
				class="flex shrink-0 items-center gap-2 rounded-md px-2.5 py-2 text-left text-xs transition-colors hover:bg-sidebar-accent {selectedProjectKey ===
				'all'
					? 'bg-sidebar-accent font-medium text-sidebar-accent-foreground'
					: 'text-muted-foreground'}"
				onclick={() => onSelect('all')}
				><DatabaseIcon size={15} /><span class="whitespace-nowrap">All projects</span></button
			>{#each projects as project (projectKey(project))}<button
					class="flex shrink-0 items-center gap-2 rounded-md px-2.5 py-2 text-left text-xs transition-colors hover:bg-sidebar-accent {selectedProjectKey ===
					projectKey(project)
						? 'bg-sidebar-accent font-medium text-sidebar-accent-foreground'
						: 'text-muted-foreground'}"
					onclick={() => onSelect(projectKey(project))}
					><FolderSimpleIcon size={15} /><span class="max-w-48 truncate"
						>{projectLabel(project)}</span
					></button
				>{/each}
		</div>
	</div>
</aside>
