<script setup lang="ts">
import { RefreshCwIcon, UsersIcon } from '@modrinth/assets'
import { Button, defineMessages, useVIntl } from '@modrinth/ui'
import { onBeforeUnmount, onMounted, ref } from 'vue'

import { TERRARIUM_SERVER_ADDRESS } from '@/helpers/terrarium-links'
import { get_server_status, type ServerStatus } from '@/helpers/worlds'

/** Раз на хвилину — сервер пінгується Server List Ping, як у списку серверів гри. */
const REFRESH_MS = 60_000

const { formatMessage } = useVIntl()

const messages = defineMessages({
	checking: { id: 'terrarium.server-status.checking', defaultMessage: 'Перевіряю…' },
	online: { id: 'terrarium.server-status.online', defaultMessage: 'Онлайн: {players} / {max}' },
	offline: { id: 'terrarium.server-status.offline', defaultMessage: 'Сервер недоступний' },
	ping: { id: 'terrarium.server-status.ping', defaultMessage: '{ping} мс' },
	refresh: { id: 'terrarium.server-status.refresh', defaultMessage: 'Оновити' },
})

const status = ref<ServerStatus | null>(null)
const online = ref<boolean | null>(null)
const loading = ref(false)
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
	<!-- Один рядок без заголовка: стан · онлайн · пінг · оновити -->
	<section class="terrarium-widget terrarium-status">
		<span
			class="terrarium-status__dot"
			:class="{ 'is-online': online === true, 'is-offline': online === false }"
			aria-hidden="true"
		/>
		<p v-if="online === null" class="terrarium-status__text text-secondary">
			{{ formatMessage(messages.checking) }}
		</p>
		<p v-else-if="online && status?.players" class="terrarium-status__text text-contrast">
			<UsersIcon class="text-brand" />
			<span class="truncate">
				{{
					formatMessage(messages.online, {
						players: status.players.online,
						max: status.players.max,
					})
				}}
			</span>
			<span v-if="status.ping !== undefined" class="terrarium-status__ping">
				{{ formatMessage(messages.ping, { ping: status.ping }) }}
			</span>
		</p>
		<p v-else class="terrarium-status__text text-secondary">
			{{ formatMessage(messages.offline) }}
		</p>
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
	</section>
</template>

<style scoped lang="scss">
@use './terrarium-widget' as *;

.terrarium-status {
	flex-direction: row;
	align-items: center;
	gap: 0.6rem;
	padding: 0.4rem 0.5rem 0.4rem 0.9rem;
}

.terrarium-status__dot {
	flex-shrink: 0;
	width: 0.6rem;
	height: 0.6rem;
	border-radius: 999px;
	background: var(--color-secondary);

	&.is-online {
		background: var(--color-green);
	}

	&.is-offline {
		background: var(--color-red);
	}
}

.terrarium-status__text {
	display: flex;
	flex: 1;
	min-width: 0;
	align-items: center;
	gap: 0.4rem;
	margin: 0;
	font-size: 0.9rem;
	font-weight: 600;

	svg {
		flex-shrink: 0;
		width: 1rem;
		height: 1rem;
	}
}

.terrarium-status__ping {
	font-size: 0.75rem;
	font-weight: 400;
	color: var(--color-secondary);
}
</style>
