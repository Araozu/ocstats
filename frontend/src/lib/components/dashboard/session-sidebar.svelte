<script lang="ts">
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDownIcon';
	import CaretUpIcon from 'phosphor-svelte/lib/CaretUpIcon';
	import type { SessionUsage } from '$lib/api/ocstats';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { getModelPricingContext } from '$lib/model-pricing';
	import { formatCost, sessionKey, shortId } from './format';

	let {
		sessions,
		projectName,
		selectedSessionKey,
		sortDirection,
		isLoading = false,
		onOverview,
		onSelect,
		onToggleSort
	}: {
		sessions: SessionUsage[];
		projectName: string;
		selectedSessionKey: string | null;
		sortDirection: 'asc' | 'desc';
		isLoading?: boolean;
		onOverview: () => void;
		onSelect: (session: SessionUsage) => void;
		onToggleSort: () => void;
	} = $props();
	const pricingStore = getModelPricingContext();

	const pricedSessions = $derived(
		sessions.map((session) => ({ session, cost: $pricingStore.totalCost(session.models) }))
	);
	let navigation: HTMLElement;

	function moveFocus(event: KeyboardEvent, index: number) {
		if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;
		event.preventDefault();
		const options = navigation.querySelectorAll<HTMLButtonElement>('[data-session-option]');
		const nextIndex =
			event.key === 'Home'
				? 0
				: event.key === 'End'
					? options.length - 1
					: Math.max(0, Math.min(options.length - 1, index + (event.key === 'ArrowDown' ? 1 : -1)));
		options[nextIndex]?.focus();
	}
</script>

<aside bind:this={navigation} class="flex h-dvh flex-col border-r bg-background">
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
	<div class="border-b p-3">
		<button
			data-session-option
			class="w-full rounded-md px-2.5 py-2 text-left text-xs transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring/50 {selectedSessionKey ===
			null
				? 'bg-primary/15 font-medium text-foreground'
				: 'text-muted-foreground'}"
			onclick={onOverview}
			onkeydown={(event) => moveFocus(event, 0)}
			aria-current={selectedSessionKey === null ? 'page' : undefined}
		>
			Overview
		</button>
	</div>
	<div class="min-h-0 flex-1 overflow-y-auto p-3">
		<div class="space-y-1">
			{#each pricedSessions as { session, cost }, index (sessionKey(session.source, session.session_id))}<button
					data-session-option
					class="w-full rounded-md border border-transparent px-2.5 py-2.5 text-left transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring/50 {selectedSessionKey ===
					sessionKey(session.source, session.session_id)
						? 'border-primary/30 bg-primary/15 shadow-sm'
						: ''}"
					onclick={() => onSelect(session)}
					onkeydown={(event) => moveFocus(event, index + 1)}
					aria-current={selectedSessionKey === sessionKey(session.source, session.session_id)
						? 'page'
						: undefined}
					><p class="truncate text-xs font-medium">{session.title || 'Untitled session'}</p>
					<div
						class="mt-1 flex items-center justify-between gap-2 text-[10px] text-muted-foreground"
					>
						<span class="truncate font-mono">{shortId(session.session_id)}</span><span
							class="shrink-0">{formatCost(cost)}</span
						>
					</div></button
				>{:else}{#if isLoading}<div
						class="space-y-2 px-2 py-1"
						aria-label="Loading sessions"
						aria-busy="true"
					>
						{#each Array.from({ length: 5 }, (_, index) => index) as index (index)}<div
								class="h-12 animate-pulse rounded-md bg-muted"
							></div>{/each}
					</div>{:else}<p class="px-2 py-3 text-xs text-muted-foreground">
						No sessions in this project.
					</p>{/if}{/each}
		</div>
	</div>
</aside>
