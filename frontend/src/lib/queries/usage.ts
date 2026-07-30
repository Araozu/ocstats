import { queryOptions } from '@tanstack/svelte-query';
import {
	getModelUsage,
	getModels,
	getPricing,
	getProjects,
	getSession,
	getSessions,
	getTurnText
} from '$lib/api/ocstats';

export const usageQueries = {
	projects: (enabled: boolean) =>
		queryOptions({ queryKey: ['projects'], queryFn: getProjects, enabled }),
	sessions: (enabled: boolean) =>
		queryOptions({ queryKey: ['sessions'], queryFn: getSessions, enabled }),
	modelUsage: (projectId: string | null, source: string | null, enabled: boolean) =>
		queryOptions({
			queryKey: ['model-usage', source, projectId],
			queryFn: () => getModelUsage(projectId ?? undefined, source ?? undefined),
			enabled
		}),
	models: (enabled: boolean) => queryOptions({ queryKey: ['models'], queryFn: getModels, enabled }),
	pricing: (enabled: boolean) =>
		queryOptions({ queryKey: ['pricing'], queryFn: getPricing, enabled }),
	session: (source: string | null, sessionId: string | null, enabled: boolean) =>
		queryOptions({
			queryKey: ['session', source, sessionId],
			queryFn: () => getSession(source!, sessionId!),
			enabled: enabled && Boolean(source && sessionId)
		}),
	turnText: (source: string, sessionId: string, turnId: string, enabled: boolean) =>
		queryOptions({
			queryKey: ['turn-text', source, sessionId, turnId],
			queryFn: () => getTurnText(source, sessionId, turnId),
			enabled,
			staleTime: Infinity
		})
};
