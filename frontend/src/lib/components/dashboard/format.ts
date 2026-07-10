import type { Project, Usage } from '$lib/api/ocstats';

export const emptyUsage: Usage = {
	cost: 0,
	input_tokens: 0,
	output_tokens: 0,
	reasoning_tokens: 0,
	cache_read_tokens: 0,
	cache_write_tokens: 0,
	total_tokens: 0
};

export function addUsage(left: Usage, right: Usage): Usage {
	return {
		cost: (left.cost ?? 0) + (right.cost ?? 0),
		input_tokens: left.input_tokens + right.input_tokens,
		output_tokens: left.output_tokens + right.output_tokens,
		reasoning_tokens: left.reasoning_tokens + right.reasoning_tokens,
		cache_read_tokens: left.cache_read_tokens + right.cache_read_tokens,
		cache_write_tokens: left.cache_write_tokens + right.cache_write_tokens,
		total_tokens: (left.total_tokens ?? 0) + (right.total_tokens ?? 0)
	};
}

export const projectKey = (project: Project) => `${project.source}:${project.id}`;
export const sessionKey = (source: string, sessionId: string) => `${source}:${sessionId}`;

export function projectLabel(project: Project) {
	return project.name?.trim() || project.worktree.split('/').filter(Boolean).pop() || project.id;
}

export function shortId(value: string) {
	return value.length > 18 ? `${value.slice(0, 8)}...${value.slice(-6)}` : value;
}

export function formatNumber(value: number | null | undefined) {
	return new Intl.NumberFormat('en-US', { notation: 'compact', maximumFractionDigits: 1 }).format(
		value ?? 0
	);
}

export function formatCost(value: number | null | undefined) {
	if (value == null) return '—';
	return new Intl.NumberFormat('en-US', {
		style: 'currency',
		currency: 'USD',
		maximumFractionDigits: 2
	}).format(value);
}

export function formatPrice(value: number | null | undefined) {
	if (value == null) return '—';
	return new Intl.NumberFormat('en-US', {
		style: 'currency',
		currency: 'USD',
		maximumFractionDigits: 3
	}).format(value);
}
