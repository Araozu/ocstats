import { getContext, setContext } from 'svelte';
import { derived, writable, type Readable } from 'svelte/store';
import type { Model, ModelPricing } from '$lib/api/ocstats';

export type PricingRate = 'input' | 'cached_read' | 'cached_write' | 'output';

type PricedModel = Pick<Model, 'provider_id' | 'model_id'>;

export type ModelPricingSnapshot = {
	find(model: PricedModel): ModelPricing | undefined;
	cost(model: PricedModel, tokens: number, rate: PricingRate): number | null;
};

export type ModelPricingStore = Readable<ModelPricingSnapshot> & {
	set(pricing: ModelPricing[]): void;
};

const MODEL_PRICING_CONTEXT = Symbol('model-pricing');

function pricingKey(provider: string, slug: string) {
	return `${provider}\0${slug}`;
}

function createSnapshot(pricing: ModelPricing[]): ModelPricingSnapshot {
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

	return {
		find,
		cost(model, tokens, rate) {
			const price = find(model)?.[rate];
			return price == null ? null : (tokens * price) / 1_000_000;
		}
	};
}

export function createModelPricingStore(): ModelPricingStore {
	const catalog = writable<ModelPricing[]>([]);
	const snapshot = derived(catalog, createSnapshot);
	return { subscribe: snapshot.subscribe, set: catalog.set };
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
