<script setup lang="ts">
import { ServerStackIcon } from '@modrinth/assets'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

import { TERRARIUM_SERVER_ADDRESS } from '@/helpers/terrarium-links'
import { get_server_status, type ServerStatus } from '@/helpers/worlds'

/** Раз на хвилину — сервер пінгується Server List Ping, як у списку серверів гри. */
const REFRESH_MS = 60_000

/**
 * Кнопка «На сервер» з онлайном усередині: вхід і стан сервера — один елемент поруч
 * із «Грати». canJoin: можна зайти зараз (збірка встановлена й оновлена, гра не запущена);
 * інакше кнопка лишається табличкою онлайну. joinDisabled: поки йде інша дія.
 */
const props = withDefaults(defineProps<{ canJoin?: boolean; joinDisabled?: boolean }>(), {
	canJoin: false,
	joinDisabled: false,
})
const emit = defineEmits<{ join: [] }>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
	join: { id: 'terrarium.hero.join-server', defaultMessage: 'На сервер' },
	server: { id: 'terrarium.server-status.server', defaultMessage: 'Сервер' },
	joinTooltip: {
		id: 'terrarium.hero.join-server-tooltip',
		defaultMessage: 'Запустити гру й одразу зайти на {address}',
	},
	checking: { id: 'terrarium.server-status.checking', defaultMessage: 'Перевіряю…' },
	players: { id: 'terrarium.server-status.players', defaultMessage: '{players} / {max} онлайн' },
	offline: { id: 'terrarium.server-status.offline', defaultMessage: 'Сервер недоступний' },
	ping: { id: 'terrarium.server-status.ping', defaultMessage: '{ping} мс' },
})

const status = ref<ServerStatus | null>(null)
const online = ref<boolean | null>(null)
const loading = ref(false)
let timer: ReturnType<typeof setInterval> | null = null

const clickable = computed(() => props.canJoin && !props.joinDisabled)

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

function onClick() {
	if (clickable.value) emit('join')
	else void refresh()
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
	<button
		v-tooltip="formatMessage(messages.joinTooltip, { address: TERRARIUM_SERVER_ADDRESS })"
		type="button"
		class="server-join"
		:class="{ 'is-join': props.canJoin, 'is-busy': props.joinDisabled }"
		:aria-disabled="!clickable"
		@click="onClick"
	>
		<ServerStackIcon class="server-join__icon" />
		<span class="server-join__text">
			<span class="server-join__title">
				{{ props.canJoin ? formatMessage(messages.join) : formatMessage(messages.server) }}
			</span>
			<span class="server-join__status">
				<span
					class="server-join__dot"
					:class="{
						'is-online': online === true,
						'is-offline': online === false,
						'animate-pulse': loading,
					}"
					aria-hidden="true"
				/>
				<template v-if="online === null">{{ formatMessage(messages.checking) }}</template>
				<template v-else-if="online && status?.players">
					{{
						formatMessage(messages.players, {
							players: status.players.online,
							max: status.players.max,
						})
					}}
					<span v-if="status.ping !== undefined" class="server-join__ping">
						· {{ formatMessage(messages.ping, { ping: status.ping }) }}
					</span>
				</template>
				<template v-else>{{ formatMessage(messages.offline) }}</template>
			</span>
		</span>
	</button>
</template>

<style scoped lang="scss">
// Висота й радіус — як у кнопок size="xl" поруч, щоб стояла з «Грати» в один ряд
.server-join {
	display: inline-flex;
	align-items: center;
	gap: 0.7rem;
	min-height: 3rem;
	padding: 0.35rem 1.1rem 0.35rem 0.9rem;
	border: 1px solid color-mix(in srgb, var(--color-contrast) 14%, transparent);
	border-radius: var(--radius-lg, 1rem);
	background: var(--terrarium-glass);
	backdrop-filter: blur(12px);
	color: var(--color-contrast);
	font: inherit;
	text-align: left;
	cursor: default;
	transition:
		border-color 0.12s ease,
		background-color 0.12s ease,
		transform 0.12s ease;

	&.is-join {
		cursor: pointer;

		&:hover {
			border-color: var(--color-green);
			background: color-mix(in srgb, var(--color-green) 14%, var(--terrarium-glass));
		}

		&:active {
			transform: scale(0.98);
		}
	}

	&.is-busy {
		cursor: default;
		opacity: 0.6;
	}
}

.server-join__icon {
	flex-shrink: 0;
	width: 1.35rem;
	height: 1.35rem;
	color: var(--color-secondary);

	.is-join & {
		color: var(--color-contrast);
	}
}

.server-join__text {
	display: flex;
	flex-direction: column;
	line-height: 1.15;
}

.server-join__title {
	font-size: 1rem;
	font-weight: 700;
}

.server-join__status {
	display: inline-flex;
	align-items: center;
	gap: 0.35rem;
	font-size: 0.75rem;
	font-weight: 600;
	color: var(--color-secondary);
	white-space: nowrap;
}

.server-join__ping {
	font-weight: 400;
}

.server-join__dot {
	flex-shrink: 0;
	width: 0.5rem;
	height: 0.5rem;
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
</style>
