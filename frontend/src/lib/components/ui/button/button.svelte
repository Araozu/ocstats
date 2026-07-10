<script lang="ts" module>
	import { cn, type WithElementRef } from '$lib/utils.js';
	import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';
	import { type VariantProps, tv } from 'tailwind-variants';

	export const buttonVariants = tv({
		base: "focus-visible:border-ring focus-visible:ring-ring/35 aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive dark:aria-invalid:border-destructive/50 rounded-md border border-transparent bg-clip-padding text-xs/relaxed font-medium tracking-[-0.01em] focus-visible:ring-2 active:not-aria-[haspopup]:translate-y-px active:not-aria-[haspopup]:shadow-none aria-invalid:ring-2 [&_svg:not([class*='size-'])]:size-4 group/button inline-flex shrink-0 items-center justify-center whitespace-nowrap transition-all outline-none select-none disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0",
		variants: {
			variant: {
				default:
					'border-primary/80 bg-[linear-gradient(to_bottom,color-mix(in_oklch,var(--primary)_88%,var(--primary-foreground)),var(--primary))] text-primary-foreground ring-1 ring-inset ring-primary-foreground/20 hover:brightness-105',
				outline:
					'border-border bg-[linear-gradient(to_bottom,color-mix(in_oklch,var(--card)_92%,var(--card-foreground)),var(--card))] text-foreground ring-1 ring-inset ring-card-foreground/5 hover:brightness-98 aria-expanded:bg-muted aria-expanded:text-foreground',
				secondary:
					'border-secondary bg-[linear-gradient(to_bottom,color-mix(in_oklch,var(--secondary)_88%,var(--secondary-foreground)),var(--secondary))] text-secondary-foreground ring-1 ring-inset ring-secondary-foreground/10 hover:brightness-98 aria-expanded:bg-secondary aria-expanded:text-secondary-foreground',
				ghost:
					'hover:border-border hover:bg-[linear-gradient(to_bottom,color-mix(in_oklch,var(--muted)_88%,var(--background)),var(--muted))] hover:text-foreground hover:ring-1 hover:ring-inset hover:ring-muted-foreground/10 aria-expanded:bg-muted aria-expanded:text-foreground',
				destructive:
					'border-destructive/30 bg-[linear-gradient(to_bottom,color-mix(in_oklch,var(--destructive)_18%,var(--background)),color-mix(in_oklch,var(--destructive)_10%,var(--background)))] text-destructive ring-1 ring-inset ring-destructive/10 hover:brightness-98 focus-visible:ring-destructive/20 dark:focus-visible:ring-destructive/40 focus-visible:border-destructive/40',
				link: 'text-primary underline-offset-4 hover:underline'
			},
			size: {
				default:
					"h-7 gap-1 px-2 text-xs/relaxed has-data-[icon=inline-end]:pr-1.5 has-data-[icon=inline-start]:pl-1.5 [&_svg:not([class*='size-'])]:size-3.5",
				xs: "h-5 gap-1 rounded-sm px-2 text-[0.625rem] has-data-[icon=inline-end]:pr-1.5 has-data-[icon=inline-start]:pl-1.5 [&_svg:not([class*='size-'])]:size-2.5",
				sm: "h-6 gap-1 px-2 text-xs/relaxed has-data-[icon=inline-end]:pr-1.5 has-data-[icon=inline-start]:pl-1.5 [&_svg:not([class*='size-'])]:size-3",
				lg: "h-8 gap-1 px-2.5 text-xs/relaxed has-data-[icon=inline-end]:pr-2 has-data-[icon=inline-start]:pl-2 [&_svg:not([class*='size-'])]:size-4",
				icon: "size-7 [&_svg:not([class*='size-'])]:size-3.5",
				'icon-xs': "size-5 rounded-sm [&_svg:not([class*='size-'])]:size-2.5",
				'icon-sm': "size-6 [&_svg:not([class*='size-'])]:size-3",
				'icon-lg': "size-8 [&_svg:not([class*='size-'])]:size-4"
			}
		},
		defaultVariants: {
			variant: 'default',
			size: 'default'
		}
	});

	export type ButtonVariant = VariantProps<typeof buttonVariants>['variant'];
	export type ButtonSize = VariantProps<typeof buttonVariants>['size'];

	export type ButtonProps = WithElementRef<HTMLButtonAttributes> &
		WithElementRef<HTMLAnchorAttributes> & {
			variant?: ButtonVariant;
			size?: ButtonSize;
		};
</script>

<script lang="ts">
	let {
		class: className,
		variant = 'default',
		size = 'default',
		ref = $bindable(null),
		href = undefined,
		type = 'button',
		disabled,
		children,
		...restProps
	}: ButtonProps = $props();
</script>

{#if href}
	<a
		bind:this={ref}
		data-slot="button"
		class={cn(buttonVariants({ variant, size }), className)}
		href={disabled ? undefined : href}
		aria-disabled={disabled}
		role={disabled ? 'link' : undefined}
		tabindex={disabled ? -1 : undefined}
		{...restProps}
	>
		{@render children?.()}
	</a>
{:else}
	<button
		bind:this={ref}
		data-slot="button"
		class={cn(buttonVariants({ variant, size }), className)}
		{type}
		{disabled}
		{...restProps}
	>
		{@render children?.()}
	</button>
{/if}
