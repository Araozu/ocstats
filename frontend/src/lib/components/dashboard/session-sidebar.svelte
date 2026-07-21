<script lang="ts">
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDownIcon';
	import CaretUpIcon from 'phosphor-svelte/lib/CaretUpIcon';
	import type { SessionUsage } from '$lib/api/ocstats';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { SvelteSet } from 'svelte/reactivity';
	import SessionTree from './session-tree.svelte';

	let {
		sessions,
		projectName,
		selectedSessionKey,
		expandedKeys,
		revealedSessionKey,
		sortDirection,
		isLoading = false,
		onOverview,
		onSelect,
		onToggleSort,
		onAncestorsRevealed
	}: {
		sessions: SessionUsage[];
		projectName: string;
		selectedSessionKey: string | null;
		expandedKeys: SvelteSet<string>;
		revealedSessionKey: string | null;
		sortDirection: 'asc' | 'desc';
		isLoading?: boolean;
		onOverview: () => void;
		onSelect: (session: SessionUsage) => void;
		onToggleSort: () => void;
		onAncestorsRevealed: (sessionKey: string) => void;
	} = $props();
</script>

<aside class="flex h-dvh flex-col border-r bg-background">
	<div class="flex h-16 items-center justify-between border-b px-4">
		<div>
			<p class="text-[11px] font-semibold uppercase tracking-[0.12em] text-muted-foreground">
				Sessions
			</p>
			<p class="mt-1 max-w-44 truncate text-xs text-muted-foreground">{projectName}</p>
		</div>
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
		instanceId="desktop"
		{sortDirection}
		{isLoading}
		{onOverview}
		{onSelect}
		{onAncestorsRevealed}
	/>
</aside>
