<script lang="ts">
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';
	import { QueryClient, QueryClientProvider } from '@tanstack/svelte-query';
	import { ModeWatcher } from 'mode-watcher';
	import { COLOR_THEME_STORAGE_KEY, DEFAULT_COLOR_THEME } from '$lib/themes';

	let { children } = $props();
	const queryClient = new QueryClient();
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>
<ModeWatcher defaultTheme={DEFAULT_COLOR_THEME} themeStorageKey={COLOR_THEME_STORAGE_KEY} />
<QueryClientProvider client={queryClient}>{@render children()}</QueryClientProvider>
