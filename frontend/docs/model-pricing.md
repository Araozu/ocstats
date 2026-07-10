# Model pricing in the frontend

Model pricing is loaded asynchronously from `GET /api/pricing`. Usage data can render before that
request succeeds, so any lookup or cost derived from pricing must remain reactive. Computing a map,
matched model, or total once during component initialization captures the initial empty catalog and
leaves the UI showing incorrect costs.

When the loaded catalog has no entry for a model used by the application, the dashboard sends
`POST /api/pricing/request` with `{ "slug": "model-slug" }`. The backend records requested slugs in
`pricing-requests.txt` in its execution directory, one slug per line, and keeps the file
deduplicated. The request is only made after the pricing catalog has loaded, so the initial empty
catalog is not mistaken for missing prices.

## Architecture

The pricing query is created once in `src/routes/+page.svelte`. That route creates a context-scoped
Svelte store and pushes every reactive query result into it:

```ts
const pricing = $derived(pricingQuery.data?.models ?? []);
const modelPricing = setModelPricingContext();
$effect(() => modelPricing.set(pricing));
```

`src/lib/model-pricing.ts` owns all shared behavior:

- rebuilding lookup indexes when the query result changes;
- matching `provider_id + model_id` against catalog `provider + slug`;
- falling back to slug-only matching for usage sources whose provider differs;
- calculating per-token costs from prices expressed per one million tokens.

Descendant components consume the store with `getModelPricingContext()`. Reading it with Svelte's
`$store` syntax makes the dependency explicit, so the initial empty snapshot is replaced everywhere
when the request completes. Do not pass pricing arrays through component props or build
component-local pricing maps.

## Correct usage

```svelte
<script lang="ts">
	import { getModelPricingContext } from '$lib/model-pricing';

	let { model } = $props();
	const pricingStore = getModelPricingContext();

	const pricing = $derived($pricingStore.find(model));
	const inputCost = $derived($pricingStore.cost(model, model.usage.input_tokens, 'input'));
</script>
```

Calls made directly from reactive template expressions also track the store correctly. For lists,
derive rows from both the usage models and `$pricingStore.find(model)`, as `model-usage-card.svelte`
does.

Session lists follow the same rule. The session usage response includes its model usage records, so
the second sidebar derives each session's cost from `$pricingStore.totalCost()` rather than reading the
raw session cost. If any model or rate is unavailable, the sidebar displays `—` until pricing is
available (or when the catalog has no matching price).

The session table and total-cost metric in the overview use the same reactive session/model totals.

The store snapshot returns `undefined` for an unmatched model and `null` when a cost cannot be
calculated. Callers must preserve that distinction until display time instead of substituting a
price of zero.

## Avoiding regressions

Do not:

- compute pricing with a plain top-level `const` from query data;
- use array length, keyed blocks, or component remounting as a reactivity signal;
- duplicate provider/slug matching in a component;
- use `0` when a catalog entry or rate is absent;
- divide by one million outside the centralized store.

When adding a new pricing consumer:

1. Call `getModelPricingContext()` during component initialization.
2. Read the returned store with `$pricingStore` inside templates or `$derived` expressions.
3. Use `$pricingStore.find()` for displaying catalog rates.
4. Use `$pricingStore.cost()` for monetary calculations.
5. Put aggregates that call the store in `$derived` expressions.
6. Verify the UI once before pricing resolves and again after it resolves.

Components using this context must be descendants of `src/routes/+page.svelte`. A test or preview that
mounts such a component in isolation must install the context with `setModelPricingContext` in a small
wrapper component.
