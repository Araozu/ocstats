import { queryOptions } from '@tanstack/svelte-query';
import {
	getModelUsage,
	getModels,
	getPricing,
	getProjects,
	getSession,
	getSessions
} from '$lib/api/ocstats';

export const usageQueries = {
	projects: (enabled: boolean) => queryOptions({ queryKey: ['projects'], queryFn: getProjects, enabled }),
	sessions: (enabled: boolean) => queryOptions({ queryKey: ['sessions'], queryFn: getSessions, enabled }),
	modelUsage: (projectId: string | null, enabled: boolean) =>
		queryOptions({
			queryKey: ['model-usage', projectId],
			queryFn: () => getModelUsage(projectId ?? undefined),
			enabled
		}),
	models: (enabled: boolean) => queryOptions({ queryKey: ['models'], queryFn: getModels, enabled }),
	pricing: (enabled: boolean) => queryOptions({ queryKey: ['pricing'], queryFn: getPricing, enabled }),
	session: (source: string | null, sessionId: string | null, enabled: boolean) =>
		queryOptions({
			queryKey: ['session', source, sessionId],
			queryFn: () => getSession(source!, sessionId!),
			enabled: enabled && Boolean(source && sessionId)
		})
};
