<script lang="ts">
	import { onMount, tick } from 'svelte';
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
	let messageElement: HTMLParagraphElement;
	let collapsible = $state(false);

	function measureOverflow() {
		if (!messageElement || expanded) return;
		collapsible = messageElement.scrollHeight > messageElement.clientHeight + 1;
	}

	onMount(() => {
		const observer = new ResizeObserver(measureOverflow);
		observer.observe(messageElement);
		measureOverflow();
		return () => observer.disconnect();
	});

	$effect(() => {
		if (!expanded) void tick().then(measureOverflow);
	});
</script>

<TableRow class="bg-primary/15 hover:bg-primary/25">
	<TableCell colspan={9} class="px-5 py-3">
		<div class="flex items-center justify-between gap-4">
			<p class="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
				User message
			</p>
			<p class="text-[11px] text-muted-foreground">
				<span class="font-medium uppercase tracking-wide">Summary</span>
				{formatCost(cost)}
			</p>
		</div>
		<p
			bind:this={messageElement}
			id={detailId}
			class="mt-1 whitespace-pre-wrap [overflow-wrap:anywhere] text-xs {expanded
				? ''
				: 'line-clamp-3'}"
		>
			{message}
		</p>
	</TableCell>
	<TableCell class="pr-5 text-right">
		{#if collapsible}<Button
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
