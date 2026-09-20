<script setup lang="ts">
import { CheckCircleIcon, DownloadIcon, FolderIcon, SpinnerIcon, XIcon } from '@modrinth/assets'
import { Button, defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { useQueryClient } from '@tanstack/vue-query'
import { onMounted, ref } from 'vue'

import {
	type ModrinthAppInfo,
	terrarium_detect_modrinth_app,
	terrarium_import_modrinth_instances,
	terrarium_use_modrinth_directory,
} from '@/helpers/terrarium'
import { instanceKeys } from '@/pages/instance/query-options'

const props = withDefaults(
	defineProps<{
		/** Банер на головній: показувати лише коли є що зробити, з кнопкою «Пізніше» */
		banner?: boolean
	}>(),
	{ banner: false },
)
const emit = defineEmits<{ dismiss: [] }>()

const { formatMessage } = useVIntl()
const { handleError, addNotification } = injectNotificationManager()
const queryClient = useQueryClient()

const messages = defineMessages({
	title: { id: 'terrarium.modrinth.title', defaultMessage: 'Modrinth App на цьому ПК' },
	found: {
		id: 'terrarium.modrinth.found',
		defaultMessage:
			'Знайдено Modrinth App: {count, plural, =0 {без збірок} one {# збірка} few {# збірки} other {# збірок}} у {path}',
	},
	notFound: {
		id: 'terrarium.modrinth.not-found',
		defaultMessage: 'Modrinth App на цьому ПК не знайдено — лаунчер використовує власну теку.',
	},
	explain: {
		id: 'terrarium.modrinth.explain',
		defaultMessage:
			'Лаунчер може використовувати теку Modrinth App замість своєї: збірки, Java і кеш будуть спільними, без дублювання на диску. Modrinth App працюватиме як і раніше — бази налаштувань у лаунчерів окремі.',
	},
	useDir: { id: 'terrarium.modrinth.use-dir', defaultMessage: 'Використовувати теку Modrinth' },
	useDirDone: {
		id: 'terrarium.modrinth.use-dir-done',
		defaultMessage: 'Теку змінено на {path}. Перезапусти лаунчер — потім можна підхопити збірки.',
	},
	shared: { id: 'terrarium.modrinth.shared', defaultMessage: 'Тека даних спільна з Modrinth App' },
	importable: {
		id: 'terrarium.modrinth.importable',
		defaultMessage:
			'{count, plural, =0 {Усі збірки Modrinth уже в бібліотеці} one {# збірка Modrinth ще не в бібліотеці} few {# збірки Modrinth ще не в бібліотеці} other {# збірок Modrinth ще не в бібліотеці}}',
	},
	import: { id: 'terrarium.modrinth.import', defaultMessage: 'Підхопити збірки' },
	imported: {
		id: 'terrarium.modrinth.imported',
		defaultMessage: 'Підхоплено {imported, plural, one {# збірку} few {# збірки} other {# збірок}}',
	},
	later: { id: 'terrarium.modrinth.later', defaultMessage: 'Пізніше' },
	restartHint: {
		id: 'terrarium.modrinth.restart-hint',
		defaultMessage: 'Зміна теки застосується після перезапуску лаунчера.',
	},
})

const info = ref<ModrinthAppInfo | null | undefined>(undefined)
const busy = ref(false)
const pendingRestart = ref(false)
const visible = ref(!props.banner)

async function load() {
	try {
		info.value = await terrarium_detect_modrinth_app()
	} catch (err) {
		info.value = null
		if (!props.banner) handleError(err)
	}
	// Банер — лише коли є реальна дія: перемкнути теку або підхопити збірки
	if (props.banner) {
		visible.value =
			!!info.value &&
			((!info.value.already_shared && info.value.instances > 0) || info.value.importable > 0)
	}
}

async function useDir() {
	busy.value = true
	try {
		const path = await terrarium_use_modrinth_directory()
		pendingRestart.value = true
		addNotification({ type: 'success', title: formatMessage(messages.useDirDone, { path }) })
	} catch (err) {
		handleError(err)
	} finally {
		busy.value = false
	}
}

async function importInstances() {
	busy.value = true
	try {
		const result = await terrarium_import_modrinth_instances()
		addNotification({
			type: 'success',
			title: formatMessage(messages.imported, { imported: result.imported }),
		})
		await queryClient.invalidateQueries({ queryKey: instanceKeys.list() })
		await load()
	} catch (err) {
		handleError(err)
	} finally {
		busy.value = false
	}
}

function dismiss() {
	visible.value = false
	emit('dismiss')
}

onMounted(load)
defineExpose({ reload: load })
</script>

<template>
	<div
		v-if="visible"
		class="rounded-xl border border-solid border-surface-5 bg-button-bg p-4"
		:class="{ 'terrarium-modrinth--banner': banner }"
	>
		<p class="m-0 flex items-center gap-2 font-medium text-contrast">
			<FolderIcon class="shrink-0 text-brand" />
			{{ formatMessage(messages.title) }}
		</p>

		<div v-if="info === undefined" class="mt-2 flex items-center gap-2 text-sm text-secondary">
			<SpinnerIcon class="animate-spin" />
		</div>

		<p v-else-if="info === null" class="m-0 mt-2 text-sm text-secondary">
			{{ formatMessage(messages.notFound) }}
		</p>

		<template v-else>
			<p class="m-0 mt-2 text-sm text-secondary">
				{{ formatMessage(messages.found, { count: info.instances, path: info.path }) }}
			</p>

			<template v-if="info.already_shared">
				<p class="m-0 mt-2 flex items-center gap-2 text-sm text-brand">
					<CheckCircleIcon class="shrink-0" /> {{ formatMessage(messages.shared) }}
				</p>
				<p class="m-0 mt-1 text-sm text-secondary">
					{{ formatMessage(messages.importable, { count: info.importable }) }}
				</p>
			</template>
			<p v-else class="m-0 mt-2 text-sm text-secondary">{{ formatMessage(messages.explain) }}</p>

			<p v-if="pendingRestart" class="m-0 mt-2 text-sm text-orange">
				{{ formatMessage(messages.restartHint) }}
			</p>

			<div class="mt-3 flex flex-wrap items-center gap-2">
				<Button
					v-if="!info.already_shared"
					type="colored"
					color="brand"
					:disabled="busy || pendingRestart"
					@click="useDir"
				>
					<SpinnerIcon v-if="busy" class="animate-spin" />
					<FolderIcon v-else />
					{{ formatMessage(messages.useDir) }}
				</Button>
				<Button
					v-else-if="info.importable > 0"
					type="colored"
					color="brand"
					:disabled="busy"
					@click="importInstances"
				>
					<SpinnerIcon v-if="busy" class="animate-spin" />
					<DownloadIcon v-else />
					{{ formatMessage(messages.import) }}
				</Button>
				<Button v-if="banner" type="transparent" :disabled="busy" @click="dismiss">
					<XIcon /> {{ formatMessage(messages.later) }}
				</Button>
			</div>
		</template>
	</div>
</template>
