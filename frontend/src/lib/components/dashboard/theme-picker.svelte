<script lang="ts">
	import PaletteIcon from 'phosphor-svelte/lib/PaletteIcon';
	import { buttonVariants } from '$lib/components/ui/button';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { DEFAULT_COLOR_THEME, colorThemes, type ColorTheme } from '$lib/themes';
	import { setTheme, theme } from 'mode-watcher';

	const selectedTheme = $derived(
		colorThemes.find((colorTheme) => colorTheme.id === theme.current)?.id ?? DEFAULT_COLOR_THEME
	);
	const selectedThemeLabel = $derived(
		colorThemes.find((colorTheme) => colorTheme.id === selectedTheme)?.label ?? 'Pastel pink'
	);

	function selectTheme(colorTheme: ColorTheme) {
		setTheme(colorTheme);
	}
</script>

<DropdownMenu.Root>
	<DropdownMenu.Trigger
		class={buttonVariants({ variant: 'outline', size: 'icon' })}
		aria-label="Change color theme"
		title={`Color theme: ${selectedThemeLabel}`}
	>
		<PaletteIcon />
		<span class="sr-only">Change color theme</span>
	</DropdownMenu.Trigger>
	<DropdownMenu.Content align="end">
		<DropdownMenu.Label>Color theme</DropdownMenu.Label>
		<DropdownMenu.RadioGroup value={selectedTheme}>
			{#each colorThemes as colorTheme (colorTheme.id)}
				<DropdownMenu.RadioItem value={colorTheme.id} onclick={() => selectTheme(colorTheme.id)}>
					<span class="flex items-center gap-2.5">
						<span class="flex gap-0.5" aria-hidden="true">
							<span class="size-2.5 rounded-full bg-primary ring-1 ring-border"></span>
							<span class="size-2.5 rounded-full bg-accent ring-1 ring-border"></span>
							<span class="size-2.5 rounded-full bg-chart-4 ring-1 ring-border"></span>
						</span>
						{colorTheme.label}
					</span>
				</DropdownMenu.RadioItem>
			{/each}
		</DropdownMenu.RadioGroup>
	</DropdownMenu.Content>
</DropdownMenu.Root>
