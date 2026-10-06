<template>
	<div
		v-if="status?.available"
		v-tooltip="tooltip"
		class="flex items-center rounded-full bg-button-bg p-1 text-sm font-semibold"
		role="radiogroup"
		aria-label="Java"
	>
		<button
			v-for="option in options"
			:key="option.value"
			type="button"
			role="radio"
			:aria-checked="option.value === current"
			:disabled="disabled || busy"
			:class="[
				'rounded-full px-3 py-1.5 transition-colors',
				option.value === current
					? 'bg-brand text-brand-inverted'
					: 'text-secondary hover:text-contrast',
				(disabled || busy) && option.value !== current ? 'cursor-not-allowed opacity-60' : '',
			]"
			@click="select(option.value)"
		>
			{{ busy && option.value === pending ? 'Завантаження…' : option.label }}
		</button>
	</div>
</template>

<script setup lang="ts">
import { injectNotificationManager } from '@modrinth/ui'
import { computed, onMounted, ref, watch } from 'vue'

import {
	type GraalvmStatus,
	terrarium_graalvm_status,
	terrarium_set_graalvm,
} from '@/helpers/terrarium'

const props = defineProps<{
	instanceId: string
	/** Гра запущена / встановлюється: перемикати не можна */
	disabled?: boolean
}>()

const { handleError } = injectNotificationManager()

const status = ref<GraalvmStatus | null>(null)
const busy = ref(false)
const pending = ref<'java' | 'graalvm' | null>(null)

const current = computed(() => (status.value?.enabled ? 'graalvm' : 'java'))
const options = computed(() => [
	{ value: 'java' as const, label: 'Java' },
	{ value: 'graalvm' as const, label: `GraalVM ${status.value?.major_version ?? ''}`.trim() },
])
const tooltip = computed(() =>
	status.value?.enabled
		? 'Збірка запускається з GraalVM: інший JIT-компілятор, у модових збірках зазвичай більше FPS і менше просідань'
		: status.value?.installed
			? 'Перемкнути збірку на GraalVM'
			: 'GraalVM: інший JIT-компілятор, зазвичай більше FPS. При першому виборі завантажиться (~350 МБ)',
)

async function refresh() {
	try {
		status.value = await terrarium_graalvm_status(props.instanceId)
	} catch (err) {
		status.value = null
		handleError(err as Error)
	}
}

async function select(value: 'java' | 'graalvm') {
	if (busy.value || props.disabled || value === current.value) return
	busy.value = true
	pending.value = value
	try {
		status.value = await terrarium_set_graalvm(props.instanceId, value === 'graalvm')
	} catch (err) {
		handleError(err as Error)
	} finally {
		busy.value = false
		pending.value = null
	}
}

onMounted(refresh)
watch(() => props.instanceId, refresh)
</script>
