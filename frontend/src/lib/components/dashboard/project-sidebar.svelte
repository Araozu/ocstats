<script lang="ts">
	import ArrowsClockwiseIcon from 'phosphor-svelte/lib/ArrowsClockwiseIcon';
	import ChartLineUpIcon from 'phosphor-svelte/lib/ChartLineUpIcon';
	import DatabaseIcon from 'phosphor-svelte/lib/DatabaseIcon';
	import FolderSimpleIcon from 'phosphor-svelte/lib/FolderSimpleIcon';
	import MonitorIcon from 'phosphor-svelte/lib/MonitorIcon';
	import MoonIcon from 'phosphor-svelte/lib/MoonIcon';
	import SunIcon from 'phosphor-svelte/lib/SunIcon';
	import type { Project } from '$lib/api/ocstats';
	import { Badge } from '$lib/components/ui/badge';
	import { Button, buttonVariants } from '$lib/components/ui/button';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { resetMode, setMode } from 'mode-watcher';
	import { projectKey, projectLabel } from './format';

	let {
		projects,
		selectedProjectKey,
		lastUpdated,
		isRefreshing = false,
		onRefresh,
		onSelect
	}: {
		projects: Project[];
		selectedProjectKey: string;
		lastUpdated: Date | null;
		isRefreshing?: boolean;
		onRefresh: () => void;
		onSelect: (key: string) => void;
	} = $props();
</script>

<aside
	class="flex flex-col border-b bg-sidebar lg:sticky lg:top-0 lg:h-screen lg:self-start lg:overflow-hidden lg:border-r lg:border-b-0"
>
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
	<div class="flex min-h-0 flex-1 flex-col p-4">
		<div class="mb-2 flex items-center justify-between px-2">
			<p class="text-[11px] font-semibold uppercase tracking-[0.12em] text-muted-foreground">
				Projects
			</p>
			<Badge variant="secondary">{projects.length}</Badge>
		</div>
		<div
			class="flex gap-1 overflow-x-auto pb-1 lg:min-h-0 lg:flex-1 lg:flex-col lg:overflow-y-auto"
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
		<div class="mt-4 border-t pt-4">
			<div class="mb-3 text-[11px] text-muted-foreground">
				{#if lastUpdated}
					Updated {lastUpdated.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
				{:else}
					Not updated yet
				{/if}
			</div>
			<div class="flex items-center gap-2">
				<Button
					variant="outline"
					size="sm"
					class="min-w-0 flex-1"
					onclick={onRefresh}
					disabled={isRefreshing}
				>
					<ArrowsClockwiseIcon class={isRefreshing ? 'animate-spin' : ''} /> Refresh
				</Button>
				<DropdownMenu.Root>
					<DropdownMenu.Trigger
						class={buttonVariants({ variant: 'outline', size: 'icon' })}
						aria-label="Change color theme"
					>
						<SunIcon class="dark:hidden" />
						<MoonIcon class="hidden dark:block" />
						<span class="sr-only">Change color theme</span>
					</DropdownMenu.Trigger>
					<DropdownMenu.Content align="end">
						<DropdownMenu.Item onclick={() => setMode('light')}><SunIcon /> Light</DropdownMenu.Item
						>
						<DropdownMenu.Item onclick={() => setMode('dark')}><MoonIcon /> Dark</DropdownMenu.Item>
						<DropdownMenu.Item onclick={resetMode}><MonitorIcon /> System</DropdownMenu.Item>
					</DropdownMenu.Content>
				</DropdownMenu.Root>
			</div>
		</div>
	</div>
</aside>
