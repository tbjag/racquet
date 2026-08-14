<script lang="ts">
	export type Member = { id: string; username: string; isSelf: boolean };

	type Props = {
		members: Member[];
	};

	let { members }: Props = $props();
</script>

<aside data-testid="member-list" class="member-list">
	<div class="center-col">
		<header class="member-list-header">
			Members <span class="count">{members.length}</span>
		</header>
		<ul>
			{#each members as m (m.id)}
				<li data-testid="member-item" class="member-item" class:is-self={m.isSelf}>
					{m.username}{m.isSelf ? ' (you)' : ''}
				</li>
			{/each}
		</ul>
	</div>
</aside>

<style>
	.member-list {
		padding: var(--space-2) var(--space-4);
		border-bottom: 1px solid var(--border);
		font-size: 0.85rem;
		color: var(--text-muted);
	}

	.member-list-header {
		display: flex;
		gap: var(--space-2);
		align-items: baseline;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		margin-bottom: var(--space-1);
	}

	.member-list .count {
		color: var(--text-muted);
		font-weight: 400;
	}

	.member-list ul {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.member-item {
		color: var(--text);
	}

	.member-item.is-self {
		color: var(--accent);
	}
</style>
