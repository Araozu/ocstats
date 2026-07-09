# Frontend Notes

## Phosphor Svelte icons

Use the `*Icon` component names from `phosphor-svelte`, for example `ChartLineUpIcon`.
The unsuffixed names are deprecated.

The project uses Phosphor's Vite Import Optimizer in `vite.config.ts`:

```ts
import { sveltePhosphorOptimize } from 'phosphor-svelte/vite';
```

Keep `sveltePhosphorOptimize()` after `sveltekit()` in the Vite plugin list. The optimizer parses JavaScript modules and must not run before SvelteKit processes raw `.svelte` files.

For explicit optimized imports, use one default import per icon:

```ts
import ChartLineUpIcon from 'phosphor-svelte/lib/ChartLineUpIcon';
```

Named imports from `phosphor-svelte` are supported, but the optimizer converts them to these per-icon default imports to improve development compile times.
