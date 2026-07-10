import { queryOptions } from '@tanstack/svelte-query';
import { getModels, getProjects, getSession, getSessions } from '$lib/api/ocstats';

export const usageQueries = {
	projects: () => queryOptions({ queryKey: ['projects'], queryFn: getProjects }),
	sessions: () => queryOptions({ queryKey: ['sessions'], queryFn: getSessions }),
	models: () => queryOptions({ queryKey: ['models'], queryFn: getModels }),
	session: (source: string | null, sessionId: string | null) =>
		queryOptions({
			queryKey: ['session', source, sessionId],
			queryFn: () => getSession(source!, sessionId!),
			enabled: Boolean(source && sessionId)
		})
};
