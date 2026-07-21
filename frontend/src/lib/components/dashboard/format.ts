import type { Project, SessionUsage, Usage } from '$lib/api/ocstats';

export type SessionSortDirection = 'asc' | 'desc';

export type SessionTreeNode = {
	key: string;
	session: SessionUsage;
	children: SessionTreeNode[];
};

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

export function sortSessionsByDate(
	sessions: SessionUsage[],
	direction: SessionSortDirection
): SessionUsage[] {
	return [...sessions].sort((left, right) => {
		const dateComparison = left.created_at_ms - right.created_at_ms;
		if (dateComparison !== 0) {
			return direction === 'asc' ? dateComparison : -dateComparison;
		}
		const keyComparison = sessionKey(left.source, left.session_id).localeCompare(
			sessionKey(right.source, right.session_id)
		);
		return direction === 'asc' ? keyComparison : -keyComparison;
	});
}

export function buildSessionTree(sessions: SessionUsage[]): SessionTreeNode[] {
	const nodes = new Map<string, SessionTreeNode>();
	const parentKeys = new Map<string, string>();

	for (const session of sessions) {
		const key = sessionKey(session.source, session.session_id);
		nodes.set(key, { key, session, children: [] });
	}

	for (const session of sessions) {
		if (!session.parent_id) continue;
		const key = sessionKey(session.source, session.session_id);
		const parentKey = sessionKey(session.source, session.parent_id);
		if (parentKey !== key && nodes.has(parentKey)) parentKeys.set(key, parentKey);
	}

	// Break one edge in every cycle so malformed data remains visible as a tree.
	for (const key of parentKeys.keys()) {
		const visited = new Set<string>();
		let current = key;
		while (parentKeys.has(current)) {
			const parentKey = parentKeys.get(current)!;
			if (visited.has(parentKey)) {
				parentKeys.delete(current);
				break;
			}
			visited.add(current);
			current = parentKey;
		}
	}

	const roots: SessionTreeNode[] = [];
	for (const node of nodes.values()) {
		const parent = nodes.get(parentKeys.get(node.key) ?? '');
		if (parent) parent.children.push(node);
		else roots.push(node);
	}
	return roots;
}

export function sessionAncestorKeys(
	roots: SessionTreeNode[],
	selectedKey: string | null
): string[] {
	if (!selectedKey) return [];

	function find(nodes: SessionTreeNode[], ancestors: string[]): string[] | null {
		for (const node of nodes) {
			if (node.key === selectedKey) return ancestors;
			const result = find(node.children, [...ancestors, node.key]);
			if (result) return result;
		}
		return null;
	}

	return find(roots, []) ?? [];
}

export function sessionRevealState(
	sessions: SessionUsage[],
	roots: SessionTreeNode[],
	selectedKey: string | null,
	revealedKey: string | null
) {
	if (!selectedKey || selectedKey === revealedKey) {
		return { ancestors: [], revealedKey: selectedKey };
	}
	const selectedExists = sessions.some(
		(session) => sessionKey(session.source, session.session_id) === selectedKey
	);
	return selectedExists
		? { ancestors: sessionAncestorKeys(roots, selectedKey), revealedKey: selectedKey }
		: { ancestors: [], revealedKey };
}

export function sessionTreeGroupId(key: string) {
	return `session-group-${encodeURIComponent(key)}`;
}

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

export function formatDateTime(value: number) {
	return new Intl.DateTimeFormat(undefined, {
		dateStyle: 'medium',
		timeStyle: 'short'
	}).format(new Date(value));
}

export function formatPrice(value: number | null | undefined) {
	if (value == null) return '—';
	return new Intl.NumberFormat('en-US', {
		style: 'currency',
		currency: 'USD',
		maximumFractionDigits: 3
	}).format(value);
}
