import { describe, expect, it } from 'vitest';
import type { Model, ModelPricing, ModelUsage, Usage } from '$lib/api/ocstats';
import { createModelPricingStore, type ModelPricingSnapshot } from './model-pricing';

const model: Model = { provider_id: 'openai', model_id: 'gpt-5.5', variant: null };
const oldTimestamp = Date.parse('2026-01-01T00:00:00Z');
const newTimestamp = Date.parse('2026-07-30T12:30:00Z');
const pricing: ModelPricing[] = [
	{
		provider: 'openai',
		slug: 'gpt-5.5',
		prices: [
			{
				effective_from: '2026-01-01T00:00:00Z',
				input: 1,
				cached_write: null,
				cached_read: 0.1,
				output: 2
			},
			{
				effective_from: '2026-07-30T12:30:00Z',
				input: 3,
				cached_write: null,
				cached_read: 0.3,
				output: 4
			}
		]
	}
];

function usage(overrides: Partial<Usage> = {}): Usage {
	return {
		cost: null,
		input_tokens: 0,
		output_tokens: 0,
		reasoning_tokens: 0,
		cache_read_tokens: 0,
		cache_write_tokens: 0,
		total_tokens: null,
		...overrides
	};
}

function modelUsage(created_at_ms: number, modelUsage: Partial<Usage>): ModelUsage {
	return { ...model, created_at_ms, usage: usage(modelUsage) };
}

describe('effective model pricing', () => {
	it('selects the latest period at or before a usage timestamp', () => {
		const store = createModelPricingStore();
		let current: ModelPricingSnapshot | undefined;
		const unsubscribe = store.subscribe((value) => (current = value));

		expect(current?.loaded).toBe(false);
		store.set(pricing);
		expect(current?.find(model, oldTimestamp)?.input).toBe(1);
		expect(current?.find(model, newTimestamp - 1)?.input).toBe(1);
		expect(current?.find(model, newTimestamp)?.input).toBe(3);
		expect(current?.find(model, oldTimestamp - 1)?.input).toBe(1);
		unsubscribe();
	});

	it('sums each usage slice using its own historical price', () => {
		const store = createModelPricingStore();
		store.set(pricing);
		let total: number | null | undefined;
		const unsubscribe = store.subscribe((snapshot) => {
			total = snapshot.totalCost([
				modelUsage(oldTimestamp, { input_tokens: 1_000_000, output_tokens: 1_000_000 }),
				modelUsage(newTimestamp, { input_tokens: 1_000_000, output_tokens: 1_000_000 })
			]);
		});

		expect(total).toBe(10);
		unsubscribe();
	});

	it('charges reasoning tokens at the output rate', () => {
		const store = createModelPricingStore();
		store.set(pricing);
		let total: number | null | undefined;
		const unsubscribe = store.subscribe((snapshot) => {
			total = snapshot.usageCost(model, usage({ reasoning_tokens: 1_000_000 }), oldTimestamp);
		});

		expect(total).toBe(2);
		unsubscribe();
	});

	it('preserves unknown rates and provider fallback semantics', () => {
		const store = createModelPricingStore();
		store.set([{ ...pricing[0], provider: 'fallback-provider' }]);
		let inputCost: number | null | undefined;
		let cacheWriteCost: number | null | undefined;
		let zeroCost: number | null | undefined;
		const unsubscribe = store.subscribe((snapshot) => {
			inputCost = snapshot.cost(model, 1_000_000, 'input', oldTimestamp);
			cacheWriteCost = snapshot.cost(model, 1_000_000, 'cached_write', oldTimestamp);
			zeroCost = snapshot.cost(model, 0, 'cached_write', oldTimestamp);
		});

		expect(inputCost).toBe(1);
		expect(cacheWriteCost).toBeNull();
		expect(zeroCost).toBe(0);
		unsubscribe();
	});

	it('normalizes legacy flat catalog entries', () => {
		const store = createModelPricingStore();
		store.set([
			{
				provider: 'openai',
				slug: 'gpt-5.5',
				input: 1,
				cached_write: null,
				cached_read: 0.1,
				output: 2
			}
		]);
		let inputCost: number | null | undefined;
		const unsubscribe = store.subscribe((snapshot) => {
			inputCost = snapshot.cost(model, 1_000_000, 'input', oldTimestamp);
		});

		expect(inputCost).toBe(1);
		unsubscribe();
	});
});
