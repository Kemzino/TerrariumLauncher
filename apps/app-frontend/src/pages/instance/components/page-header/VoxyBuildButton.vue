<template>
	<div
		v-if="status?.compatible"
		v-tooltip="tooltip"
		class="relative flex items-center overflow-hidden rounded-full bg-button-bg p-1 text-sm font-semibold"
	>
		<div
			v-if="status.building"
			class="absolute inset-y-0 left-0 bg-brand-highlight transition-all"
			:style="{ width: `${Math.round(status.progress * 100)}%` }"
		/>
		<button
			type="button"
			:disabled="disabled || status.building || (status.installed !== null && status.up_to_date)"
			:class="[
				'relative rounded-full px-3 py-1.5 transition-colors',
				status.installed && status.up_to_date && !status.building
					? 'bg-brand text-brand-inverted'
					: 'text-secondary hover:text-contrast',
				disabled && !status.building ? 'cursor-not-allowed opacity-60' : '',
			]"
			@click="build"
		>
			{{ label }}
		</button>
		<button
			v-if="status.building"
			v-tooltip="'Скасувати збірку'"
			type="button"
			:disabled="cancelling"
			class="relative rounded-full px-2 py-1.5 text-secondary transition-colors hover:text-contrast"
			:class="cancelling ? 'cursor-not-allowed opacity-60' : ''"
			aria-label="Скасувати збірку Voxy"
			@click="cancel"
		>
			✕
		</button>
		<button
			v-if="status.installed && !status.building"
			type="button"
			:disabled="disabled"
			class="relative rounded-full px-2 py-1.5 text-secondary transition-colors hover:text-contrast"
			:class="disabled ? 'cursor-not-allowed opacity-60' : ''"
			aria-label="Прибрати Voxy"
			@click="remove"
		>
			✕
		</button>
	</div>
</template>

<script setup lang="ts">
import { injectNotificationManager } from '@modrinth/ui'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'

import {
	terrarium_voxy_build,
	terrarium_voxy_cancel,
	terrarium_voxy_remove,
	terrarium_voxy_status,
	VOXY_CANCELLED,
	type VoxyStatus,
} from '@/helpers/terrarium'

const props = defineProps<{
	instanceId: string
	/** Гра запущена / встановлюється: чіпати mods/ не можна */
	disabled?: boolean
}>()

const { handleError, addNotification } = injectNotificationManager()

const status = ref<VoxyStatus | null>(null)
const cancelling = ref(false)
let poll: ReturnType<typeof setInterval> | null = null

const label = computed(() => {
	const s = status.value
	if (!s) return 'Voxy'
	if (s.building) return `Voxy: ${s.stage ?? 'збірка'} ${Math.round(s.progress * 100)}%`
	if (s.installed && s.up_to_date) return 'Voxy ✓'
	if (s.installed) return 'Оновити Voxy'
	return 'Зібрати Voxy'
})

const tooltip = computed(() => {
	const s = status.value
	if (!s) return ''
	const commit = `${s.repo}@${s.commit.slice(0, 7)}`
	if (s.building)
		return `Збирається з ${commit}. Перший раз — кілька хвилин (качаються JDK і Minecraft для збірки)`
	if (s.installed && s.up_to_date) return `Voxy зібрано з ${commit}: ${s.installed}. ✕ — прибрати`
	if (s.installed) return `Встановлено ${s.installed}; зібрати з ${commit}`
	return `Voxy (далека прорисовка): лаунчер скачає код ${commit}, збере мод і покладе в mods. Git і Java не потрібні`
})

function startPolling() {
	if (poll) return
	poll = setInterval(async () => {
		try {
			status.value = await terrarium_voxy_status(props.instanceId)
		} catch {
			/* наступна спроба */
		}
	}, 1000)
}

function stopPolling() {
	if (poll) clearInterval(poll)
	poll = null
}

async function refresh() {
	try {
		status.value = await terrarium_voxy_status(props.instanceId)
		if (status.value.building) startPolling()
	} catch (err) {
		status.value = null
		handleError(err as Error)
	}
}

async function build() {
	if (!status.value || status.value.building || props.disabled) return
	status.value = { ...status.value, building: true, stage: 'Початок', progress: 0 }
	startPolling()
	try {
		status.value = await terrarium_voxy_build(props.instanceId)
		addNotification({ type: 'success', title: 'Voxy зібрано і встановлено' })
	} catch (err) {
		if (String((err as { message?: string })?.message ?? err).includes(VOXY_CANCELLED)) {
			addNotification({ type: 'info', title: VOXY_CANCELLED })
		} else {
			handleError(err as Error)
		}
		await refresh()
	} finally {
		cancelling.value = false
		stopPolling()
	}
}

async function cancel() {
	if (cancelling.value) return
	cancelling.value = true
	try {
		await terrarium_voxy_cancel()
	} catch (err) {
		cancelling.value = false
		handleError(err as Error)
	}
}

async function remove() {
	if (props.disabled) return
	try {
		status.value = await terrarium_voxy_remove(props.instanceId)
	} catch (err) {
		handleError(err as Error)
	}
}

onMounted(refresh)
onUnmounted(stopPolling)
watch(() => props.instanceId, refresh)
</script>
