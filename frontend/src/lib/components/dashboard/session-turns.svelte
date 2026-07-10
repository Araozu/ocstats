<script lang="ts">
	import type { SessionDetail, Turn } from '$lib/api/ocstats';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { getModelPricingContext } from '$lib/model-pricing';
	import {
		Table,
		TableBody,
		TableCell,
		TableHead,
		TableHeader,
		TableRow
	} from '$lib/components/ui/table';
	import SessionTurnRow from './session-turn-row.svelte';
	import SessionUserMessageRow from './session-user-message-row.svelte';

	let { session }: { session: SessionDetail } = $props();
	const pricingStore = getModelPricingContext();
	let expandedMessages = $state<string[]>([]);

	function toggleMessage(message: string) {
		expandedMessages = expandedMessages.includes(message)
			? expandedMessages.filter((item) => item !== message)
			: [...expandedMessages, message];
	}

	function turnCost(turn: Turn) {
		if (!turn.model) return null;
		let total = 0;
		for (const [tokens, rate] of [
			[turn.usage.input_tokens, 'input'],
			[turn.usage.cache_read_tokens, 'cached_read'],
			[turn.usage.output_tokens, 'output']
		] as const) {
			const cost = $pricingStore.cost(turn.model, tokens, rate);
			if (cost == null && tokens > 0) return null;
			total += cost ?? 0;
		}
		return total;
	}

	function userMessageCost(startIndex: number) {
		const message = session.turns[startIndex]?.user_message;
		if (!message) return null;
		let total = 0;
		for (let index = startIndex; index < session.turns.length; index += 1) {
			const turn = session.turns[index];
			if (turn.user_message !== message) break;
			const cost = turnCost(turn);
			if (cost == null) return null;
			total += cost;
		}
		return total;
	}
</script>

<Card class="!overflow-visible">
	<CardHeader
		><CardTitle>Turns</CardTitle>
		<p class="mt-1 text-xs text-muted-foreground">
			Token usage from each completed turn in this session.
		</p></CardHeader
	>
	<CardContent class="p-0">
		<Table containerClass="!overflow-visible">
			<TableHeader
				><TableRow
					><TableHead class="hidden pl-5 sm:table-cell">Turn</TableHead><TableHead>Model</TableHead
					><TableHead class="hidden sm:table-cell">Activity</TableHead><TableHead>Input</TableHead
					><TableHead class="hidden sm:table-cell">Cached</TableHead><TableHead>Output</TableHead
					><TableHead class="pr-5 text-right">Pricing</TableHead></TableRow
				></TableHeader
			>
			<TableBody>
				{#each session.turns as turn, index (turn.id)}
					{@const isMessageStart =
						turn.user_message &&
						(index === 0 || session.turns[index - 1].user_message !== turn.user_message)}
					{#if isMessageStart}{@const message = turn.user_message!}<SessionUserMessageRow
							{message}
							cost={userMessageCost(index)}
							expanded={expandedMessages.includes(message)}
							onToggle={() => toggleMessage(message)}
						/>{/if}
					<SessionTurnRow {turn} {index} />
				{:else}<TableRow
						><TableCell colspan={7} class="h-24 text-center text-muted-foreground"
							>No completed turns.</TableCell
						></TableRow
					>{/each}
			</TableBody>
		</Table>
	</CardContent>
</Card>
