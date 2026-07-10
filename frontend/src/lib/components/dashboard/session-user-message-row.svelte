<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { TableCell, TableRow } from '$lib/components/ui/table';
	import { formatCost } from './format';
	let {
		message,
		cost,
		expanded,
		onToggle
	}: { message: string; cost: number | null; expanded: boolean; onToggle: () => void } = $props();
</script>

<TableRow class="bg-primary/15 hover:bg-primary/25">
	<TableCell colspan={7} class="px-5 py-3">
		<div class="flex items-center justify-between gap-4">
			<p class="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
				User message
			</p>
			<p class="text-[11px] text-muted-foreground">
				<span class="font-medium uppercase tracking-wide">Summary</span>
				{formatCost(cost)}
			</p>
		</div>
		<p class="mt-1 whitespace-pre-wrap text-xs {expanded ? '' : 'line-clamp-3'}">{message}</p>
		{#if message.length > 240}<Button
				variant="ghost"
				size="xs"
				class="mt-2"
				onclick={onToggle}
				aria-expanded={expanded}>{expanded ? 'Show less' : 'Show more'}</Button
			>{/if}
	</TableCell>
</TableRow>
