import { describe, expect, it } from 'vitest';
import type { SessionUsage } from '$lib/api/ocstats';
import { sortSessionsByDate } from './format';

function session(source: string, id: string, createdAt: number): SessionUsage {
	return {
		source,
		session_id: id,
		project_id: 'project-1',
		title: id,
		created_at_ms: createdAt,
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
