<script setup lang="ts">
import { ChevronDownIcon, RefreshCwIcon, ServerIcon, UsersIcon } from '@modrinth/assets'
import { Button, defineMessages, useVIntl } from '@modrinth/ui'
import { onBeforeUnmount, onMounted, ref } from 'vue'

import { TERRARIUM_SERVER_ADDRESS } from '@/helpers/terrarium-links'
import { get_server_status, type ServerStatus } from '@/helpers/worlds'

/** Раз на хвилину — сервер пінгується Server List Ping, як у списку серверів гри. */
const REFRESH_MS = 60_000

const { formatMessage } = useVIntl()

const messages = defineMessages({
	title: { id: 'terrarium.server-status.title', defaultMessage: 'Сервер' },
	checking: { id: 'terrarium.server-status.checking', defaultMessage: 'Перевіряю…' },
	online: { id: 'terrarium.server-status.online', defaultMessage: 'Онлайн: {players} / {max}' },
	offline: { id: 'terrarium.server-status.offline', defaultMessage: 'Сервер недоступний' },
	ping: { id: 'terrarium.server-status.ping', defaultMessage: '{ping} мс' },
	noSample: {
		id: 'terrarium.server-status.no-sample',
		defaultMessage: 'Сервер не показує список гравців',
	},
	nobody: { id: 'terrarium.server-status.nobody', defaultMessage: 'Зараз нікого немає' },
	showPlayers: { id: 'terrarium.server-status.show-players', defaultMessage: 'Показати гравців' },
	hidePlayers: { id: 'terrarium.server-status.hide-players', defaultMessage: 'Сховати гравців' },
	refresh: { id: 'terrarium.server-status.refresh', defaultMessage: 'Оновити' },
})

const status = ref<ServerStatus | null>(null)
const online = ref<boolean | null>(null)
const loading = ref(false)
const expanded = ref(false)
let timer: ReturnType<typeof setInterval> | null = null

async function refresh() {
	if (loading.value) return
	loading.value = true
	try {
		status.value = await get_server_status(TERRARIUM_SERVER_ADDRESS)
		online.value = true
	} catch {
		// Недоступний — лишаємо останні відомі цифри, але показуємо офлайн
		online.value = false
	} finally {
		loading.value = false
	}
}

onMounted(() => {
	void refresh()
	timer = setInterval(() => void refresh(), REFRESH_MS)
})
onBeforeUnmount(() => {
	if (timer) clearInterval(timer)
})
</script>

<template>
	<section class="terrarium-widget">
		<header class="terrarium-widget__head">
			<h3 class="terrarium-widget__title"><ServerIcon /> {{ formatMessage(messages.title) }}</h3>
			<span class="flex items-center gap-1">
				<span
					v-if="online !== null"
					class="terrarium-status__dot"
					:class="online ? 'is-online' : 'is-offline'"
					aria-hidden="true"
				/>
				<Button
					v-tooltip="formatMessage(messages.refresh)"
					type="transparent"
					size="sm"
					icon-only
					:disabled="loading"
					:aria-label="formatMessage(messages.refresh)"
					@click="refresh"
				>
					<RefreshCwIcon :class="{ 'animate-spin': loading }" />
				</Button>
			</span>
		</header>
		<div class="terrarium-widget__body">
			<p v-if="online === null" class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.checking) }}
			</p>
			<template v-else-if="online && status?.players">
				<p class="m-0 flex items-center gap-2 font-semibold text-contrast">
					<UsersIcon class="text-brand" />
					{{
						formatMessage(messages.online, {
							players: status.players.online,
							max: status.players.max,
						})
					}}
					<span v-if="status.ping !== undefined" class="terrarium-status__ping">
						{{ formatMessage(messages.ping, { ping: status.ping }) }}
					</span>
				</p>
				<Button
					v-if="status.players.online > 0"
					size="sm"
					type="transparent"
					class="mt-1 !px-0"
					@click="expanded = !expanded"
				>
					<ChevronDownIcon class="transition-transform" :class="{ '-rotate-90': !expanded }" />
					{{ expanded ? formatMessage(messages.hidePlayers) : formatMessage(messages.showPlayers) }}
				</Button>
				<p v-else class="m-0 mt-1 text-sm text-secondary">{{ formatMessage(messages.nobody) }}</p>
				<ul v-if="expanded && status.players.sample.length" class="terrarium-status__players">
					<li v-for="player in status.players.sample" :key="player.id || player.name">
						{{ player.name }}
					</li>
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

.terrarium-status__ping {
	font-size: 0.75rem;
	font-weight: 400;
	color: var(--color-secondary);
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
