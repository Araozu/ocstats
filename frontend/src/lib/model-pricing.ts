import { getContext, setContext } from 'svelte';
import { derived, writable, type Readable } from 'svelte/store';
import type { Model, ModelPricing, ModelUsage } from '$lib/api/ocstats';

export type PricingRate = 'input' | 'cached_read' | 'cached_write' | 'output';

type PricedModel = Pick<Model, 'provider_id' | 'model_id'>;

export type ModelPricingSnapshot = {
	loaded: boolean;
	find(model: PricedModel): ModelPricing | undefined;
	cost(model: PricedModel, tokens: number, rate: PricingRate): number | null;
	totalCost(models: ModelUsage[]): number | null;
};

export type ModelPricingStore = Readable<ModelPricingSnapshot> & {
	set(pricing: ModelPricing[], loaded?: boolean): void;
};

const MODEL_PRICING_CONTEXT = Symbol('model-pricing');

function pricingKey(provider: string, slug: string) {
	return `${provider}\0${slug}`;
}

function createSnapshot([pricing, loaded]: [ModelPricing[], boolean]): ModelPricingSnapshot {
	const byProviderAndSlug = new Map<string, ModelPricing>();
	const bySlug = new Map<string, ModelPricing>();

	for (const item of pricing) {
		byProviderAndSlug.set(pricingKey(item.provider, item.slug), item);
		if (!bySlug.has(item.slug)) bySlug.set(item.slug, item);
	}

	function find(model: PricedModel) {
		return (
			byProviderAndSlug.get(pricingKey(model.provider_id, model.model_id)) ??
			bySlug.get(model.model_id)
		);
	}

	function thisCost(model: PricedModel, tokens: number, rate: PricingRate) {
		const price = find(model)?.[rate];
		return price == null ? null : (tokens * price) / 1_000_000;
	}

	function totalCost(models: ModelUsage[]) {
		if (models.length === 0) return null;
		let total = 0;
		for (const model of models) {
			for (const [tokens, rate] of [
				[model.usage.input_tokens, 'input'],
				[model.usage.cache_read_tokens, 'cached_read'],
				[model.usage.output_tokens, 'output']
			] as const) {
				if (tokens === 0) continue;
				const modelCost = thisCost(model, tokens, rate);
				if (modelCost == null) return null;
				total += modelCost;
			}
		}
		return total;
	}

	return {
		loaded,
		find,
		cost(model, tokens, rate) {
			return thisCost(model, tokens, rate);
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
