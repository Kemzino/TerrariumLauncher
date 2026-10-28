<script setup lang="ts">
import { PlayIcon, RefreshCwIcon, ServerStackIcon, UsersIcon } from '@modrinth/assets'
import { Button, defineMessages, useVIntl } from '@modrinth/ui'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

import { TERRARIUM_SERVER_ADDRESS } from '@/helpers/terrarium-links'
import { get_server_status, type ServerStatus } from '@/helpers/worlds'

/** Раз на хвилину — сервер пінгується Server List Ping, як у списку серверів гри. */
const REFRESH_MS = 60_000

/**
 * canJoin: показати «На сервер» (запуск гри одразу на сервер); joinDisabled: поки йде інша дія.
 * variant: card — картка, «На сервер» під станом; pill — лише смужка стану без кнопки;
 * lobby — велика картка сервера: онлайн великими цифрами, заповненість і головна кнопка входу.
 */
const props = withDefaults(
	defineProps<{
		canJoin?: boolean
		joinDisabled?: boolean
		variant?: 'card' | 'pill' | 'lobby'
	}>(),
	{
		canJoin: false,
		joinDisabled: false,
		variant: 'card',
	},
)
const emit = defineEmits<{ join: [] }>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
	checking: { id: 'terrarium.server-status.checking', defaultMessage: 'Перевіряю…' },
	online: { id: 'terrarium.server-status.online', defaultMessage: 'Онлайн: {players} / {max}' },
	offline: { id: 'terrarium.server-status.offline', defaultMessage: 'Сервер недоступний' },
	ping: { id: 'terrarium.server-status.ping', defaultMessage: '{ping} мс' },
	refresh: { id: 'terrarium.server-status.refresh', defaultMessage: 'Оновити' },
	join: { id: 'terrarium.hero.join-server', defaultMessage: 'На сервер' },
	joinBig: { id: 'terrarium.server-status.join-big', defaultMessage: 'Грати на сервері' },
	joinTooltip: {
		id: 'terrarium.hero.join-server-tooltip',
		defaultMessage: 'Запустити гру й одразу зайти на {address}',
	},
	lobbyOnline: { id: 'terrarium.server-status.lobby-online', defaultMessage: 'Сервер онлайн' },
	lobbyPlayers: { id: 'terrarium.server-status.lobby-players', defaultMessage: 'гравців зараз' },
})

const status = ref<ServerStatus | null>(null)
const online = ref<boolean | null>(null)
const loading = ref(false)
let timer: ReturnType<typeof setInterval> | null = null

/** Заповненість сервера 0..1 — смужка у великій картці */
const fill = computed(() => {
	const p = status.value?.players
	return p && p.max > 0 ? Math.min(1, p.online / p.max) : 0
})

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
	<!-- Велика картка сервера: що на сервері зараз і одна головна кнопка -->
	<section v-if="props.variant === 'lobby'" class="terrarium-widget terrarium-lobby">
		<header class="terrarium-lobby__head">
			<span
				class="terrarium-status__dot"
				:class="{ 'is-online': online === true, 'is-offline': online === false }"
				aria-hidden="true"
			/>
			<span class="terrarium-lobby__label">
				{{
					online === null
						? formatMessage(messages.checking)
						: online
							? formatMessage(messages.lobbyOnline)
							: formatMessage(messages.offline)
				}}
			</span>
			<span v-if="online && status?.ping !== undefined" class="terrarium-status__ping">
				{{ formatMessage(messages.ping, { ping: status.ping }) }}
			</span>
			<Button
				v-tooltip="formatMessage(messages.refresh)"
				class="ml-auto"
				type="transparent"
				size="sm"
				icon-only
				:disabled="loading"
				:aria-label="formatMessage(messages.refresh)"
				@click="refresh"
			>
				<RefreshCwIcon :class="{ 'animate-spin': loading }" />
			</Button>
		</header>
		<div v-if="status?.players" class="terrarium-lobby__count" :class="{ 'is-stale': !online }">
			<strong>{{ status.players.online }}</strong>
			<span>/ {{ status.players.max }} {{ formatMessage(messages.lobbyPlayers) }}</span>
		</div>
		<div v-if="status?.players" class="terrarium-lobby__fill" aria-hidden="true">
			<div :style="{ width: `${Math.max(2, Math.round(fill * 100))}%` }" />
		</div>
		<p class="terrarium-lobby__address">{{ TERRARIUM_SERVER_ADDRESS }}</p>
		<Button
			v-if="props.canJoin"
			type="colored"
			color="brand"
			size="xl"
			class="terrarium-lobby__join"
			:disabled="props.joinDisabled"
			@click="emit('join')"
		>
			<PlayIcon /> {{ formatMessage(messages.joinBig) }}
		</Button>
	</section>

	<!-- Картка (стан, під ним «На сервер») або тонка смужка стану -->
	<section
		v-else
		class="terrarium-status"
		:class="props.variant === 'pill' ? 'terrarium-status--pill' : 'terrarium-widget'"
	>
		<div class="terrarium-status__row">
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
		</div>
		<Button
			v-if="props.canJoin && props.variant === 'card'"
			v-tooltip="formatMessage(messages.joinTooltip, { address: TERRARIUM_SERVER_ADDRESS })"
			type="colored"
			color="brand"
			class="terrarium-status__join"
			:disabled="props.joinDisabled"
			@click="emit('join')"
		>
			<ServerStackIcon /> {{ formatMessage(messages.join) }}
		</Button>
	</section>
</template>

<style scoped lang="scss">
@use './terrarium-widget' as *;

.terrarium-status {
	gap: 0.35rem;
	min-width: 15rem;
	padding: 0.35rem 0.5rem 0.5rem;
}

.terrarium-status--pill {
	display: flex;
	min-width: 0;
	padding: 0.15rem 0.25rem 0.15rem 0.5rem;
	border: 1px solid color-mix(in srgb, var(--color-contrast) 12%, transparent);
	border-radius: 999px;
	background: var(--terrarium-glass);
	backdrop-filter: blur(14px);
}

.terrarium-status__row {
	display: flex;
	align-items: center;
	gap: 0.6rem;
	padding-left: 0.4rem;
}

.terrarium-status__join {
	width: 100%;
	justify-content: center;
}

.terrarium-status__dot {
	flex-shrink: 0;
	width: 0.6rem;
	height: 0.6rem;
	border-radius: 999px;
	background: var(--color-secondary);

	&.is-online {
		background: var(--color-green);
		box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-green) 25%, transparent);
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

.terrarium-lobby {
	gap: 0.5rem;
	width: 100%;
	padding: 0.9rem 1.1rem 1.1rem;
}

.terrarium-lobby__head {
	display: flex;
	align-items: center;
	gap: 0.55rem;
}

.terrarium-lobby__label {
	font-size: 0.78rem;
	font-weight: 700;
	letter-spacing: 0.08em;
	text-transform: uppercase;
	color: var(--color-contrast);
}

.terrarium-lobby__count {
	display: flex;
	align-items: baseline;
	gap: 0.5rem;
	color: var(--color-secondary);

	strong {
		font-size: 3rem;
		line-height: 1;
		font-weight: 800;
		color: var(--color-contrast);
		font-variant-numeric: tabular-nums;
	}

	&.is-stale strong {
		color: var(--color-secondary);
	}
}

.terrarium-lobby__fill {
	height: 6px;
	border-radius: 999px;
	background: color-mix(in srgb, var(--color-contrast) 12%, transparent);
	overflow: hidden;

	div {
		height: 100%;
		border-radius: inherit;
		background: var(--color-green);
		transition: width 0.4s ease;
	}
}

.terrarium-lobby__address {
	margin: 0;
	font-size: 0.8rem;
	color: var(--color-secondary);
	font-family: var(--mono-font, monospace);
}

.terrarium-lobby__join {
	width: 100%;
	justify-content: center;
	margin-top: 0.35rem;
	box-shadow: 0 8px 30px var(--color-brand-shadow);
}
</style>
