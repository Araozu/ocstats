<script lang="ts">
	import { formatCost, formatNumber } from './format';
	let { tokens, cost }: { tokens: number; cost: number | null } = $props();
</script>

<p>{formatNumber(tokens)}</p>
<p class="text-[11px] text-muted-foreground">{formatCost(cost)}</p>
