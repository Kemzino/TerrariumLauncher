<script setup lang="ts">
import { ChevronDownIcon, ServerIcon, UsersIcon } from '@modrinth/assets'
import { Button, defineMessages, useVIntl } from '@modrinth/ui'
import { ref } from 'vue'

/**
 * Заглушка онлайну сервера. Коли буде адреса сервера — сюди піде Server List
 * Ping (кількість, максимум, sample гравців); контракт — `ServerStatus`.
 */
export interface ServerStatus {
	online: boolean
	players: number
	max_players: number
	/** Гравці онлайн (сервер віддає до 12 імен, якщо не сховано) */
	sample: { name: string }[]
	motd?: string
}

const { formatMessage } = useVIntl()

const messages = defineMessages({
	title: { id: 'terrarium.server-status.title', defaultMessage: 'Сервер' },
	unknown: {
		id: 'terrarium.server-status.unknown',
		defaultMessage: 'Онлайн сервера підключиться найближчим часом.',
	},
	online: {
		id: 'terrarium.server-status.online',
		defaultMessage: 'Онлайн: {players} / {max}',
	},
	offline: { id: 'terrarium.server-status.offline', defaultMessage: 'Сервер недоступний' },
	players: { id: 'terrarium.server-status.players', defaultMessage: 'Хто онлайн' },
	noSample: {
		id: 'terrarium.server-status.no-sample',
		defaultMessage: 'Сервер не показує список гравців',
	},
	showPlayers: { id: 'terrarium.server-status.show-players', defaultMessage: 'Показати гравців' },
	hidePlayers: { id: 'terrarium.server-status.hide-players', defaultMessage: 'Сховати гравців' },
})

// Поки джерела немає — null; компонент уже вміє рендерити статус і список
const status = ref<ServerStatus | null>(null)
const expanded = ref(false)
</script>

<template>
	<section class="terrarium-widget">
		<header class="terrarium-widget__head">
			<h3 class="terrarium-widget__title"><ServerIcon /> {{ formatMessage(messages.title) }}</h3>
			<span
				v-if="status"
				class="terrarium-status__dot"
				:class="status.online ? 'is-online' : 'is-offline'"
				aria-hidden="true"
			/>
		</header>
		<div class="terrarium-widget__body">
			<div v-if="!status" class="terrarium-widget__empty">
				<p class="m-0">{{ formatMessage(messages.unknown) }}</p>
			</div>
			<template v-else-if="status.online">
				<p class="m-0 flex items-center gap-2 font-semibold text-contrast">
					<UsersIcon class="text-brand" />
					{{ formatMessage(messages.online, { players: status.players, max: status.max_players }) }}
				</p>
				<Button size="sm" type="transparent" class="mt-2" @click="expanded = !expanded">
					<ChevronDownIcon class="transition-transform" :class="{ '-rotate-90': !expanded }" />
					{{ expanded ? formatMessage(messages.hidePlayers) : formatMessage(messages.showPlayers) }}
				</Button>
				<ul v-if="expanded && status.sample.length" class="terrarium-status__players">
					<li v-for="player in status.sample" :key="player.name">{{ player.name }}</li>
				</ul>
				<p v-else-if="expanded" class="m-0 mt-1 text-sm text-secondary">
					{{ formatMessage(messages.noSample) }}
				</p>
			</template>
			<p v-else class="m-0 text-secondary">{{ formatMessage(messages.offline) }}</p>
		</div>
	</section>
</template>

<style scoped lang="scss">
@use './terrarium-widget' as *;

.terrarium-status__dot {
	width: 0.6rem;
	height: 0.6rem;
	border-radius: 999px;

	&.is-online {
		background: var(--color-green);
	}

	&.is-offline {
		background: var(--color-red);
	}
}

.terrarium-status__players {
	display: flex;
	flex-wrap: wrap;
	gap: 0.35rem;
	margin: 0.5rem 0 0;
	padding: 0;
	list-style: none;

	li {
		padding: 0.15rem 0.6rem;
		border-radius: 999px;
		background: var(--color-button-bg);
		font-size: 0.85rem;
		color: var(--color-contrast);
	}
}
</style>
