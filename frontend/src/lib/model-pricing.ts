import { getContext, setContext } from 'svelte';
import { derived, writable, type Readable } from 'svelte/store';
import type { Model, ModelPricing, ModelUsage, PricePeriod } from '$lib/api/ocstats';

export type PricingRate = 'input' | 'cached_read' | 'cached_write' | 'output';

type PricedModel = Pick<Model, 'provider_id' | 'model_id'>;

export type ModelPricingSnapshot = {
	loaded: boolean;
	history(model: PricedModel): ModelPricing | undefined;
	find(model: PricedModel, atMs: number): PricePeriod | undefined;
	cost(model: PricedModel, tokens: number, rate: PricingRate, atMs: number): number | null;
	totalCost(models: ModelUsage[]): number | null;
};

export type ModelPricingStore = Readable<ModelPricingSnapshot> & {
	set(pricing: ModelPricing[], loaded?: boolean): void;
};

const MODEL_PRICING_CONTEXT = Symbol('model-pricing');
const LEGACY_EFFECTIVE_FROM = '1970-01-01T00:00:00Z';

function pricingKey(provider: string, slug: string) {
	return `${provider}\0${slug}`;
}

function createSnapshot([pricing, loaded]: [ModelPricing[], boolean]): ModelPricingSnapshot {
	const byProviderAndSlug = new Map<string, ModelPricing>();
	const bySlug = new Map<string, ModelPricing>();

	for (const rawItem of pricing) {
		const item =
			rawItem.prices?.length || rawItem.input == null || rawItem.output == null
				? rawItem
				: {
						...rawItem,
						prices: [
							{
								effective_from: LEGACY_EFFECTIVE_FROM,
								input: rawItem.input,
								cached_write: rawItem.cached_write ?? null,
								cached_read: rawItem.cached_read ?? null,
								output: rawItem.output
							}
						]
					};
		byProviderAndSlug.set(pricingKey(item.provider, item.slug), item);
		if (!bySlug.has(item.slug)) bySlug.set(item.slug, item);
	}

	function history(model: PricedModel) {
		return (
			byProviderAndSlug.get(pricingKey(model.provider_id, model.model_id)) ??
			bySlug.get(model.model_id)
		);
	}

	function find(model: PricedModel, atMs: number) {
		const modelHistory = history(model);
		let selected: PricePeriod | undefined;
		let selectedAt = -Infinity;
		for (const price of modelHistory?.prices ?? []) {
			const effectiveAt = Date.parse(price.effective_from);
			if (!Number.isNaN(effectiveAt) && effectiveAt <= atMs && effectiveAt > selectedAt) {
				selected = price;
				selectedAt = effectiveAt;
			}
		}
		return selected;
	}

	function thisCost(model: PricedModel, tokens: number, rate: PricingRate, atMs: number) {
		if (tokens === 0) return 0;
		const price = find(model, atMs)?.[rate];
		return price == null ? null : (tokens * price) / 1_000_000;
	}

	function totalCost(models: ModelUsage[]) {
		if (models.length === 0) return null;
		let total = 0;
		for (const model of models) {
			for (const [tokens, rate] of [
				[model.usage.input_tokens, 'input'],
				[model.usage.cache_read_tokens, 'cached_read'],
				[model.usage.cache_write_tokens, 'cached_write'],
				[model.usage.output_tokens, 'output']
			] as const) {
				if (tokens === 0) continue;
				const modelCost = thisCost(model, tokens, rate, model.created_at_ms);
				if (modelCost == null) return null;
				total += modelCost;
			}
		}
		return total;
	}

	return {
		loaded,
		history,
		find,
		cost(model, tokens, rate, atMs) {
			return thisCost(model, tokens, rate, atMs);
		},
		totalCost
	};
}

export function createModelPricingStore(): ModelPricingStore {
	const catalog = writable<[ModelPricing[], boolean]>([[], false]);
	const snapshot = derived(catalog, createSnapshot);
	return {
		subscribe: snapshot.subscribe,
		set(pricing, loaded = true) {
			catalog.set([pricing, loaded]);
		}
	};
}

export function setModelPricingContext(): ModelPricingStore {
	const store = createModelPricingStore();

	setContext(MODEL_PRICING_CONTEXT, store);
	return store;
}

export function getModelPricingContext() {
	const store = getContext<ModelPricingStore | undefined>(MODEL_PRICING_CONTEXT);
	if (!store) throw new Error('Model pricing context has not been initialized.');
	return store;
}
