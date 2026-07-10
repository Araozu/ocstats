<script lang="ts">
	import CircleNotchIcon from 'phosphor-svelte/lib/CircleNotchIcon';
	import type { SessionUsage } from '$lib/api/ocstats';
	import { Badge } from '$lib/components/ui/badge';
	import { formatCost, sessionKey, shortId } from './format';

	let {
		sessions,
		projectName,
		selectedSessionKey,
		isLoading = false,
		onOverview,
		onSelect
	}: {
		sessions: SessionUsage[];
		projectName: string;
		selectedSessionKey: string | null;
		isLoading?: boolean;
		onOverview: () => void;
		onSelect: (session: SessionUsage) => void;
	} = $props();
</script>

<aside
	class="flex flex-col border-b bg-background lg:sticky lg:top-0 lg:h-screen lg:self-start lg:overflow-hidden lg:border-r lg:border-b-0"
>
	<div class="flex h-16 items-center justify-between border-b px-4">
		<div>
			<p class="text-[11px] font-semibold uppercase tracking-[0.12em] text-muted-foreground">
				Sessions
			</p>
			<p class="mt-1 max-w-44 truncate text-xs text-muted-foreground">{projectName}</p>
		</div>
		<Badge variant="secondary">{sessions.length}</Badge>
	</div>
	<div class="border-b p-3">
		<button
			class="w-full rounded-md px-2.5 py-2 text-left text-xs transition-colors hover:bg-muted {selectedSessionKey ===
			null
				? 'bg-muted font-medium'
				: 'text-muted-foreground'}"
			onclick={onOverview}
		>
			Overview
		</button>
	</div>
	<div class="max-h-72 min-h-0 overflow-y-auto p-3 lg:flex-1 lg:max-h-none">
		<div class="space-y-1">
			{#each sessions as session (sessionKey(session.source, session.session_id))}<button
					class="w-full rounded-md border border-transparent px-2.5 py-2.5 text-left transition-colors hover:bg-muted {selectedSessionKey ===
					sessionKey(session.source, session.session_id)
						? 'border-border bg-muted'
						: ''}"
					onclick={() => onSelect(session)}
					><p class="truncate text-xs font-medium">{session.title || 'Untitled session'}</p>
					<div
						class="mt-1 flex items-center justify-between gap-2 text-[10px] text-muted-foreground"
					>
						<span class="truncate font-mono">{shortId(session.session_id)}</span><span
							class="shrink-0">{formatCost(session.usage.cost)}</span
						>
					</div></button
				>{:else}{#if isLoading}<div
						class="flex items-center gap-2 px-2 py-3 text-xs text-muted-foreground"
					>
						<CircleNotchIcon class="animate-spin" size={14} /> Loading sessions
					</div>{:else}<p class="px-2 py-3 text-xs text-muted-foreground">
						No sessions in this project.
					</p>{/if}{/each}
		</div>
	</div>
</aside>
