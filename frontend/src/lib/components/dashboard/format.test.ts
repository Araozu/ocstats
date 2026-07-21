import { describe, expect, it } from 'vitest';
import type { SessionUsage } from '$lib/api/ocstats';
import {
	buildSessionTree,
	sessionAncestorKeys,
	sessionRevealState,
	sortSessionsByDate
} from './format';

function session(
	source: string,
	id: string,
	createdAt: number,
	parentId: string | null = null
): SessionUsage {
	return {
		source,
		session_id: id,
		project_id: 'project-1',
		title: id,
		created_at_ms: createdAt,
		parent_id: parentId,
		usage: {
			cost: null,
			input_tokens: 0,
			output_tokens: 0,
			reasoning_tokens: 0,
			cache_read_tokens: 0,
			cache_write_tokens: 0,
			total_tokens: null
		},
		models: [],
		source_kind: 'messages'
	};
}

describe('sortSessionsByDate', () => {
	const sessions = [
		session('/data/a.db', 'newest', 30),
		session('/data/b.db', 'oldest', 10),
		session('/data/c.db', 'middle', 20)
	];

	it('sorts sessions from oldest to newest', () => {
		expect(sortSessionsByDate(sessions, 'asc').map((item) => item.session_id)).toEqual([
			'oldest',
			'middle',
			'newest'
		]);
	});

	it('sorts sessions from newest to oldest without mutating the input', () => {
		expect(sortSessionsByDate(sessions, 'desc').map((item) => item.session_id)).toEqual([
			'newest',
			'middle',
			'oldest'
		]);
		expect(sessions.map((item) => item.session_id)).toEqual(['newest', 'oldest', 'middle']);
	});

	it('uses the session key as a deterministic tie breaker', () => {
		const tied = [session('/data/b.db', 'same', 10), session('/data/a.db', 'same', 10)];
		expect(sortSessionsByDate(tied, 'asc').map((item) => item.source)).toEqual([
			'/data/a.db',
			'/data/b.db'
		]);
	});
});

describe('buildSessionTree', () => {
	it('supports nested children and keeps orphans visible as roots', () => {
		const root = session('/data/opencode.db', 'root', 10);
		const child = session('/data/opencode.db', 'child', 20, 'root');
		const grandchild = session('/data/opencode.db', 'grandchild', 30, 'child');
		const orphan = session('/data/opencode.db', 'orphan', 40, 'missing');

		const roots = buildSessionTree([root, child, grandchild, orphan]);

		expect(roots.map((node) => node.key)).toEqual([
			'/data/opencode.db:root',
			'/data/opencode.db:orphan'
		]);
		expect(roots[0].children[0].children[0].session.session_id).toBe('grandchild');
	});

	it('breaks cycles without dropping sessions', () => {
		const first = session('/data/opencode.db', 'first', 10, 'second');
		const second = session('/data/opencode.db', 'second', 20, 'first');

		const roots = buildSessionTree([first, second]);

		expect(roots).toHaveLength(1);
		expect(roots[0].children).toHaveLength(1);
	});

	it('finds every ancestor needed to reveal a selected descendant', () => {
		const root = session('/data/opencode.db', 'root', 10);
		const child = session('/data/opencode.db', 'child', 20, 'root');
		const grandchild = session('/data/opencode.db', 'grandchild', 30, 'child');
		const roots = buildSessionTree([root, child, grandchild]);

		expect(sessionAncestorKeys(roots, '/data/opencode.db:grandchild')).toEqual([
			'/data/opencode.db:root',
			'/data/opencode.db:child'
		]);
	});

	it('does not reveal a collapsed branch again for the same selection', () => {
		const root = session('/data/opencode.db', 'root', 10);
		const child = session('/data/opencode.db', 'child', 20, 'root');
		const sessions = [root, child];
		const roots = buildSessionTree(sessions);
		const first = sessionRevealState(sessions, roots, '/data/opencode.db:child', null);
		const afterCollapse = sessionRevealState(
			sessions,
			roots,
			'/data/opencode.db:child',
			first.revealedKey
		);

		expect(first.ancestors).toEqual(['/data/opencode.db:root']);
		expect(afterCollapse.ancestors).toEqual([]);
		expect(afterCollapse.revealedKey).toBe(first.revealedKey);
	});
});
