<script lang="ts">
	import ArrowsClockwiseIcon from 'phosphor-svelte/lib/ArrowsClockwiseIcon';
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDownIcon';
	import CaretUpIcon from 'phosphor-svelte/lib/CaretUpIcon';
	import ChartLineUpIcon from 'phosphor-svelte/lib/ChartLineUpIcon';
	import DatabaseIcon from 'phosphor-svelte/lib/DatabaseIcon';
	import FolderSimpleIcon from 'phosphor-svelte/lib/FolderSimpleIcon';
	import ListIcon from 'phosphor-svelte/lib/ListIcon';
	import MonitorIcon from 'phosphor-svelte/lib/MonitorIcon';
	import MoonIcon from 'phosphor-svelte/lib/MoonIcon';
	import SunIcon from 'phosphor-svelte/lib/SunIcon';
	import SortAscendingIcon from 'phosphor-svelte/lib/SortAscendingIcon';
	import type { Project, SessionUsage } from '$lib/api/ocstats';
	import { Badge } from '$lib/components/ui/badge';
	import { Button, buttonVariants } from '$lib/components/ui/button';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import * as Sheet from '$lib/components/ui/sheet';
	import { resetMode, setMode } from 'mode-watcher';
	import { SvelteSet } from 'svelte/reactivity';
	import { projectKey, projectLabel, type ProjectSortMode } from './format';
	import SessionTree from './session-tree.svelte';
	import ThemePicker from './theme-picker.svelte';

	let {
		projects,
		sessions,
		projectName,
		selectedProjectKey,
		selectedSessionKey,
		expandedKeys,
		revealedSessionKey,
		sortDirection,
		isLoading = false,
		isImporting = false,
		onImport,
		onOverview,
		onProjectSelect,
		projectSortMode,
		onProjectSortModeChange,
		onSessionSelect,
		onToggleSort,
		onAncestorsRevealed
	}: {
		projects: Project[];
		sessions: SessionUsage[];
		projectName: string;
		selectedProjectKey: string;
		selectedSessionKey: string | null;
		expandedKeys: SvelteSet<string>;
		revealedSessionKey: string | null;
		sortDirection: 'asc' | 'desc';
		isLoading?: boolean;
		isImporting?: boolean;
		onImport: () => void;
		onOverview: () => void;
		onProjectSelect: (key: string) => void;
		projectSortMode: ProjectSortMode;
		onProjectSortModeChange: (mode: ProjectSortMode) => void;
		onSessionSelect: (session: SessionUsage) => void;
		onToggleSort: () => void;
		onAncestorsRevealed: (sessionKey: string) => void;
	} = $props();
	let open = $state(false);

	function selectOverview() {
		onOverview();
		open = false;
	}

	function selectProject(key: string) {
		onProjectSelect(key);
		open = false;
	}

	function selectSession(session: SessionUsage) {
		onSessionSelect(session);
		open = false;
	}

	function closeAtDesktop() {
		if (window.matchMedia('(min-width: 80rem)').matches) open = false;
	}
</script>

<svelte:window onresize={closeAtDesktop} />

<header
	class="sticky top-0 z-40 flex h-14 min-w-0 items-center gap-2 border-b bg-background/95 px-3 backdrop-blur-sm xl:hidden"
>
	<Sheet.Root bind:open>
		<Sheet.Trigger aria-label="Open dashboard navigation">
			{#snippet child({ props })}
				<Button variant="outline" size="icon-lg" class="size-9" {...props}>
					<ListIcon />
				</Button>
			{/snippet}
		</Sheet.Trigger>
		<Sheet.Content side="left" class="p-0">
			<Sheet.Header class="border-b pr-14">
				<div class="flex items-center gap-2">
					<div
						class="flex size-8 items-center justify-center rounded-lg bg-primary text-primary-foreground"
					>
						<ChartLineUpIcon size={18} weight="bold" />
					</div>
					<div class="min-w-0">
						<Sheet.Title>ocstats</Sheet.Title>
						<Sheet.Description class="truncate">{projectName}</Sheet.Description>
					</div>
				</div>
			</Sheet.Header>

			<ScrollArea class="min-h-0 flex-1">
				<nav class="space-y-5 p-4" aria-label="Dashboard navigation">
					<section aria-labelledby="mobile-projects-heading">
						<div class="mb-2 flex items-center justify-between px-2">
							<p
								id="mobile-projects-heading"
								class="text-[11px] font-semibold uppercase tracking-[0.12em] text-muted-foreground"
							>
								Projects
							</p>
							<div class="flex items-center gap-1">
								<Badge variant="secondary">{projects.length}</Badge>
								<DropdownMenu.Root>
									<DropdownMenu.Trigger
										class={buttonVariants({ variant: 'ghost', size: 'icon-sm' })}
										aria-label="Sort projects"
										title={projectSortMode === 'name'
											? 'Projects sorted by name'
											: 'Projects sorted by most recent'}
									>
										<SortAscendingIcon />
									</DropdownMenu.Trigger>
									<DropdownMenu.Content align="end">
										<DropdownMenu.Label>Sort projects</DropdownMenu.Label>
										<DropdownMenu.RadioGroup value={projectSortMode}>
											<DropdownMenu.RadioItem
												value="name"
												onclick={() => onProjectSortModeChange('name')}>Name</DropdownMenu.RadioItem
											>
											<DropdownMenu.RadioItem
												value="recent"
												onclick={() => onProjectSortModeChange('recent')}
												>Most recent</DropdownMenu.RadioItem
											>
										</DropdownMenu.RadioGroup>
									</DropdownMenu.Content>
								</DropdownMenu.Root>
							</div>
						</div>
						<div class="space-y-1">
							<button
								class="flex min-h-10 w-full items-center gap-2 rounded-md px-2.5 py-2 text-left text-xs transition-colors hover:bg-muted {selectedProjectKey ===
								'all'
									? 'bg-primary/15 font-medium text-foreground'
									: 'text-muted-foreground'}"
								onclick={() => selectProject('all')}
								aria-pressed={selectedProjectKey === 'all'}
							>
								<DatabaseIcon class="shrink-0" size={15} /> All projects
							</button>
							{#each projects as project (projectKey(project))}
								<button
									class="flex min-h-10 w-full items-center gap-2 rounded-md px-2.5 py-2 text-left text-xs transition-colors hover:bg-muted {selectedProjectKey ===
									projectKey(project)
										? 'bg-primary/15 font-medium text-foreground'
										: 'text-muted-foreground'}"
									onclick={() => selectProject(projectKey(project))}
									aria-pressed={selectedProjectKey === projectKey(project)}
								>
									<FolderSimpleIcon class="shrink-0" size={15} />
									<span class="min-w-0 break-words">{projectLabel(project)}</span>
								</button>
							{/each}
						</div>
					</section>

					<section aria-labelledby="mobile-sessions-heading" aria-busy={isLoading}>
						<div class="mb-2 flex items-center justify-between px-2">
							<p
								id="mobile-sessions-heading"
								class="text-[11px] font-semibold uppercase tracking-[0.12em] text-muted-foreground"
							>
								Sessions
							</p>
							<div class="flex items-center gap-1">
								<Badge variant="secondary">{sessions.length}</Badge>
								<Button
									variant="ghost"
									size="icon-sm"
									onclick={onToggleSort}
									aria-label={sortDirection === 'desc'
										? 'Sort sessions oldest first'
										: 'Sort sessions newest first'}
									title={sortDirection === 'desc' ? 'Oldest first' : 'Newest first'}
								>
									{#if sortDirection === 'desc'}<CaretDownIcon />{:else}<CaretUpIcon />{/if}
								</Button>
							</div>
						</div>
						<SessionTree
							{sessions}
							{selectedSessionKey}
							{expandedKeys}
							{revealedSessionKey}
							instanceId="mobile"
							{sortDirection}
							{isLoading}
							mobile
							onOverview={selectOverview}
							onSelect={selectSession}
							{onAncestorsRevealed}
						/>
					</section>
				</nav>
			</ScrollArea>
		</Sheet.Content>
	</Sheet.Root>

	<div class="min-w-0 flex-1">
		<p class="truncate text-sm font-semibold">{projectName}</p>
		<p class="text-[10px] text-muted-foreground">{sessions.length} sessions</p>
	</div>
	<Button
		variant="outline"
		size="icon-lg"
		class="size-9"
		onclick={onImport}
		disabled={isImporting}
		title="Import OpenCode data"
		aria-label="Import OpenCode data"
	>
		<ArrowsClockwiseIcon class={isImporting ? 'animate-spin' : ''} />
	</Button>
	<DropdownMenu.Root>
		<DropdownMenu.Trigger
			class={buttonVariants({ variant: 'outline', size: 'icon-lg', class: 'size-9' })}
			aria-label="Change light or dark mode"
		>
			<SunIcon class="dark:hidden" />
			<MoonIcon class="hidden dark:block" />
		</DropdownMenu.Trigger>
		<DropdownMenu.Content align="end">
			<DropdownMenu.Item onclick={() => setMode('light')}><SunIcon /> Light</DropdownMenu.Item>
			<DropdownMenu.Item onclick={() => setMode('dark')}><MoonIcon /> Dark</DropdownMenu.Item>
			<DropdownMenu.Item onclick={resetMode}><MonitorIcon /> System</DropdownMenu.Item>
		</DropdownMenu.Content>
	</DropdownMenu.Root>
	<ThemePicker />
</header>
