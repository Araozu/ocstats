<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDownIcon';
	import { TableCell, TableRow } from '$lib/components/ui/table';
	import { formatCost } from './format';
	let {
		message,
		messageKey,
		cost,
		expanded,
		onToggle
	}: {
		message: string;
		messageKey: string;
		cost: number | null;
		expanded: boolean;
		onToggle: () => void;
	} = $props();
	const detailId = $derived(`user-message-${messageKey.replace(/[^a-zA-Z0-9_-]/g, '-')}`);
</script>

<TableRow class="bg-primary/15 hover:bg-primary/25">
	<TableCell colspan={8} class="px-5 py-3">
		<div class="flex items-center justify-between gap-4">
			<p class="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
				User message
			</p>
			<p class="text-[11px] text-muted-foreground">
				<span class="font-medium uppercase tracking-wide">Summary</span>
				{formatCost(cost)}
			</p>
		</div>
		<p id={detailId} class="mt-1 whitespace-pre-wrap text-xs {expanded ? '' : 'line-clamp-3'}">
			{message}
		</p>
	</TableCell>
	<TableCell class="pr-5 text-right">
		{#if message.length > 240}<Button
				variant="ghost"
				size="xs"
				onclick={onToggle}
				aria-expanded={expanded}
				aria-controls={detailId}
			>
				<CaretDownIcon
					class={expanded ? 'rotate-180 transition-transform' : 'transition-transform'}
					aria-hidden="true"
				/>{expanded ? 'Show less' : 'Show more'}
			</Button>{/if}
	</TableCell>
</TableRow>
