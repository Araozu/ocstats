<script lang="ts">
	import ArrowsClockwiseIcon from 'phosphor-svelte/lib/ArrowsClockwiseIcon';
	import MonitorIcon from 'phosphor-svelte/lib/MonitorIcon';
	import MoonIcon from 'phosphor-svelte/lib/MoonIcon';
	import SunIcon from 'phosphor-svelte/lib/SunIcon';
	import { resetMode, setMode } from 'mode-watcher';
	import { Button, buttonVariants } from '$lib/components/ui/button';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';

	let {
		title,
		isSession = false,
		isRefreshing = false,
		lastUpdated,
		onRefresh
	}: {
		title: string;
		isSession?: boolean;
		isRefreshing?: boolean;
		lastUpdated: Date | null;
		onRefresh: () => void;
	} = $props();
</script>

<header
	class="flex min-h-16 flex-wrap items-center justify-between gap-3 border-b px-5 py-3 md:px-8"
>
	<div>
		<p class="text-xs text-muted-foreground">{isSession ? 'Session details' : 'Usage overview'}</p>
		<h1 class="mt-0.5 text-lg font-semibold tracking-tight">{title}</h1>
	</div>
	<div class="flex items-center gap-2">
		{#if lastUpdated}<span class="hidden text-[11px] text-muted-foreground sm:inline"
				>Updated {lastUpdated.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</span
			>{/if}
		<Button variant="outline" size="sm" onclick={onRefresh} disabled={isRefreshing}
			><ArrowsClockwiseIcon class={isRefreshing ? 'animate-spin' : ''} /> Refresh</Button
		>
		<DropdownMenu.Root
			><DropdownMenu.Trigger
				class={buttonVariants({ variant: 'outline', size: 'icon' })}
				aria-label="Change color theme"
				><SunIcon class="dark:hidden" /><MoonIcon class="hidden dark:block" /><span class="sr-only"
					>Change color theme</span
				></DropdownMenu.Trigger
			><DropdownMenu.Content align="end"
				><DropdownMenu.Item onclick={() => setMode('light')}><SunIcon /> Light</DropdownMenu.Item
				><DropdownMenu.Item onclick={() => setMode('dark')}><MoonIcon /> Dark</DropdownMenu.Item
				><DropdownMenu.Item onclick={resetMode}><MonitorIcon /> System</DropdownMenu.Item
				></DropdownMenu.Content
			></DropdownMenu.Root
		>
	</div>
</header>
