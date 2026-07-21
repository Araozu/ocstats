<script lang="ts">
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDownIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';
	import type { SessionUsage } from '$lib/api/ocstats';
	import { getModelPricingContext } from '$lib/model-pricing';
	import { SvelteSet } from 'svelte/reactivity';
	import {
		buildSessionTree,
		formatCost,
		sessionKey,
		sessionRevealState,
		sessionTreeGroupId,
		sortSessionsByDate,
		shortId,
		type SessionSortDirection,
		type SessionTreeNode
	} from './format';

	let {
		sessions,
		selectedSessionKey,
		expandedKeys,
		revealedSessionKey,
		instanceId,
		sortDirection,
		isLoading = false,
		mobile = false,
		onOverview,
		onSelect,
		onAncestorsRevealed
	}: {
		sessions: SessionUsage[];
		selectedSessionKey: string | null;
		expandedKeys: SvelteSet<string>;
		revealedSessionKey: string | null;
		instanceId: string;
		sortDirection: SessionSortDirection;
		isLoading?: boolean;
		mobile?: boolean;
		onOverview: () => void;
		onSelect: (session: SessionUsage) => void;
		onAncestorsRevealed: (sessionKey: string) => void;
	} = $props();

	const pricingStore = getModelPricingContext();
	let navigation: HTMLElement;
	const orderedSessions = $derived(sortSessionsByDate(sessions, sortDirection));
	const roots = $derived(buildSessionTree(orderedSessions));

	$effect(() => {
		const reveal = sessionRevealState(
			orderedSessions,
			roots,
			selectedSessionKey,
			revealedSessionKey
		);
		for (const key of reveal.ancestors) {
			expandedKeys.add(key);
		}
		if (reveal.revealedKey !== revealedSessionKey && reveal.revealedKey !== null) {
			onAncestorsRevealed(reveal.revealedKey);
		}
	});

	function toggleExpanded(key: string) {
		if (expandedKeys.has(key)) expandedKeys.delete(key);
		else expandedKeys.add(key);
	}

	function domId(key: string) {
		return `${instanceId}-${sessionTreeGroupId(key)}`;
	}

	function moveFocus(event: KeyboardEvent) {
		if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;
		event.preventDefault();
		const options = Array.from(
			navigation.querySelectorAll<HTMLButtonElement>('[data-session-option]')
		).filter((option) => !option.closest('[hidden]'));
		const current = event.currentTarget as HTMLButtonElement;
		const index = options.indexOf(current);
		if (index === -1 || !options.length) return;
		const nextIndex =
			event.key === 'Home'
				? 0
				: event.key === 'End'
					? options.length - 1
					: Math.max(0, Math.min(options.length - 1, index + (event.key === 'ArrowDown' ? 1 : -1)));
		options[nextIndex]?.focus();
	}
</script>

{#snippet renderNodes(nodes: SessionTreeNode[])}
	{#each nodes as node (node.key)}
		{@const expanded = expandedKeys.has(node.key)}
		{@const rowId = `${domId(node.key)}-row`}
		{@const groupId = domId(node.key)}
		{@const cost = $pricingStore.totalCost(node.session.models)}
		<div>
			<div class="flex min-w-0 items-start gap-1">
				{#if node.children.length}
					<button
						type="button"
						class="mt-2 flex size-5 shrink-0 items-center justify-center rounded-sm text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50"
						onclick={() => toggleExpanded(node.key)}
						aria-expanded={expanded}
						aria-controls={groupId}
						aria-label={`${expanded ? 'Collapse' : 'Expand'} ${node.children.length} subagent ${node.children.length === 1 ? 'session' : 'sessions'} under ${node.session.title || 'Untitled session'} (${shortId(node.session.session_id)})`}
					>
						{#if expanded}<CaretDownIcon size={14} />{:else}<CaretRightIcon size={14} />{/if}
					</button>
				{:else}
					<span class="size-5 shrink-0" aria-hidden="true"></span>
				{/if}
				<button
					id={rowId}
					data-session-option
					type="button"
					class="min-w-0 flex-1 rounded-md border border-transparent px-2.5 py-2.5 text-left transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring/50 {mobile
						? 'min-h-12'
						: ''} {selectedSessionKey === sessionKey(node.session.source, node.session.session_id)
						? 'border-primary/30 bg-primary/15 shadow-sm'
						: ''}"
					onclick={() => onSelect(node.session)}
					onkeydown={moveFocus}
					aria-current={selectedSessionKey ===
					sessionKey(node.session.source, node.session.session_id)
						? 'page'
						: undefined}
				>
					<p class="text-xs font-medium {mobile ? 'break-words' : 'truncate'}">
						{node.session.title || 'Untitled session'}
					</p>
					<div
						class="mt-1 flex items-center justify-between gap-2 text-[10px] text-muted-foreground"
					>
						<span class="truncate font-mono">{shortId(node.session.session_id)}</span><span
							class="shrink-0">{formatCost(cost)}</span
						>
					</div>
				</button>
			</div>
			{#if node.children.length}
				<div
					id={groupId}
					role="group"
					aria-labelledby={rowId}
					aria-label={`${node.children.length} subagent ${node.children.length === 1 ? 'session' : 'sessions'}`}
					hidden={!expanded}
					class="ml-3 space-y-1 border-l border-border pl-2"
				>
					{@render renderNodes(node.children)}
				</div>
			{/if}
		</div>
	{/each}
{/snippet}

<div
	bind:this={navigation}
	class={mobile ? 'space-y-1' : 'min-h-0 flex-1 overflow-y-auto p-3'}
	aria-busy={isLoading}
>
	<div class={mobile ? '' : 'border-b p-3'}>
		<button
			data-session-option
			type="button"
			class="w-full rounded-md px-2.5 py-2 text-left text-xs transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring/50 {selectedSessionKey ===
			null
				? 'bg-primary/15 font-medium text-foreground'
				: 'text-muted-foreground'}"
			onclick={onOverview}
			onkeydown={moveFocus}
			aria-current={selectedSessionKey === null ? 'page' : undefined}
		>
			Overview
		</button>
	</div>
	{#if isLoading}
		<div class="space-y-2 px-2 py-1" aria-label="Loading sessions" aria-busy="true">
			{#each Array.from({ length: 5 }, (_, index) => index) as index (index)}
				<div class="h-12 animate-pulse rounded-md bg-muted"></div>
			{/each}
		</div>
	{:else if sessions.length}
		<div class={mobile ? 'space-y-1' : 'mt-3 space-y-1'}>
			{@render renderNodes(roots)}
		</div>
	{:else}
		<p class="px-2 py-3 text-xs text-muted-foreground">No sessions in this project.</p>
	{/if}
</div>
