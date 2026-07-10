const API_URL = 'http://127.0.0.1:4117/api';

export type Project = { source: string; id: string; name: string | null; worktree: string };

export type Usage = {
	cost: number | null;
	input_tokens: number;
	output_tokens: number;
	reasoning_tokens: number;
	cache_read_tokens: number;
	cache_write_tokens: number;
	total_tokens: number | null;
};

export type SessionUsage = {
	source: string;
	session_id: string;
	project_id: string;
	title: string;
	usage: Usage;
	source_kind: string;
};

export type Model = { provider_id: string; model_id: string; variant: string | null };
export type ModelUsage = Model & { usage: Usage };
export type SessionDetail = SessionUsage & { models: ModelUsage[] };
export type ModelPricing = {
	provider: string;
	slug: string;
	input: number;
	cached_write: number | null;
	cached_read: number | null;
	output: number;
};
export type PricingCatalog = { models: ModelPricing[] };

async function get<T>(path: string): Promise<T> {
	const response = await fetch(`${API_URL}${path}`);
	if (!response.ok) throw new Error('The analytics service returned an error.');
	return response.json() as Promise<T>;
}

export const getProjects = () => get<Project[]>('/projects');
export const getSessions = () => get<SessionUsage[]>('/usage/sessions');
export const getModelUsage = (projectId?: string) =>
	get<ModelUsage[]>(`/usage/models${projectId ? `?project_id=${encodeURIComponent(projectId)}` : ''}`);
export const getModels = () => get<Model[]>('/models');
export const getPricing = () => get<PricingCatalog>('/pricing');

export function getSession(source: string, sessionId: string) {
	const params = new URLSearchParams({ source, session_id: sessionId });
	return get<SessionDetail>(`/usage/session?${params}`);
}
