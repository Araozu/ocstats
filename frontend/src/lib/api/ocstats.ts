const API_URL = '/api';

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
	models: ModelUsage[];
	source_kind: string;
};

export type Model = { provider_id: string; model_id: string; variant: string | null };
export type ModelUsage = Model & { usage: Usage };
export type Turn = {
	id: string;
	message_id: string;
	model: Model | null;
	user_message: string | null;
	types: string[];
	reason: string | null;
	usage: Usage;
	created_at_ms: number;
	updated_at_ms: number;
};
export type TurnText = { turn_id: string; message_id: string; text: string | null };
export type SessionDetail = SessionUsage & { models: ModelUsage[]; turns: Turn[] };
export type ModelPricing = {
	provider: string;
	slug: string;
	input: number;
	cached_write: number | null;
	cached_read: number | null;
	output: number;
};
export type PricingCatalog = { models: ModelPricing[] };
export type AuthStatus = { authenticated: boolean };
export type ImportSummary = {
	sessions: number;
	assistant_messages: number;
	steps: number;
	issues: number;
};

async function get<T>(path: string): Promise<T> {
	const response = await fetch(`${API_URL}${path}`);
	if (!response.ok) throw new Error('The analytics service returned an error.');
	return response.json() as Promise<T>;
}

export const getProjects = () => get<Project[]>('/projects');
export const getSessions = () => get<SessionUsage[]>('/usage/sessions');
export const getModelUsage = (projectId?: string) =>
	get<ModelUsage[]>(
		`/usage/models${projectId ? `?project_id=${encodeURIComponent(projectId)}` : ''}`
	);
export const getModels = () => get<Model[]>('/models');
export const getPricing = () => get<PricingCatalog>('/pricing');
export const getAuthStatus = () => get<AuthStatus>('/auth/status');

export async function importData(): Promise<ImportSummary> {
	const response = await fetch(`${API_URL}/import`, { method: 'POST' });
	if (!response.ok) throw new Error('The analytics service returned an error.');
	return response.json() as Promise<ImportSummary>;
}

export async function login(password: string): Promise<void> {
	const response = await fetch(`${API_URL}/auth/login`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ password })
	});
	if (!response.ok) throw new Error('Incorrect password.');
}

export async function requestPricing(slug: string): Promise<void> {
	const response = await fetch(`${API_URL}/pricing/request`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ slug })
	});
	if (!response.ok) throw new Error('The analytics service returned an error.');
}

export function getSession(source: string, sessionId: string) {
	const params = new URLSearchParams({ source, session_id: sessionId });
	return get<SessionDetail>(`/usage/session?${params}`);
}

export function getTurnText(source: string, sessionId: string, turnId: string) {
	const params = new URLSearchParams({ source, session_id: sessionId, turn_id: turnId });
	return get<TurnText>(`/usage/turn-text?${params}`);
}
