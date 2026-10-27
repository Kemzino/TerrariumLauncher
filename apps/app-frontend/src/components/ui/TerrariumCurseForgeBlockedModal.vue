<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.header)"
		max-width="720px"
		:on-hide="onHide"
	>
		<div class="flex flex-col gap-3">
			<p class="m-0 text-sm text-secondary">{{ formatMessage(messages.hint) }}</p>
			<p v-if="downloadsDir" class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.watching, { dir: downloadsDir }) }}
			</p>
			<p v-else class="m-0 text-sm text-orange">{{ formatMessage(messages.noDownloadsDir) }}</p>
			<div class="flex max-h-[55vh] flex-col gap-2 overflow-y-auto pr-1">
				<article
					v-for="file in files"
					:key="file.file_id"
					class="flex flex-wrap items-center gap-2 rounded-xl border border-solid border-surface-5 bg-button-bg p-3"
				>
					<div class="flex min-w-0 grow flex-col gap-1">
						<span class="font-semibold text-contrast">{{ file.name }}</span>
						<span class="break-all text-sm text-secondary">{{ file.file_name }}</span>
						<span v-if="errors[file.file_id]" class="text-sm text-red">
							{{ errors[file.file_id] }}
						</span>
					</div>
					<span
						v-if="status[file.file_id] === 'done'"
						class="flex items-center gap-1 text-sm font-semibold text-green"
					>
						<CheckIcon /> {{ formatMessage(messages.done) }}
					</span>
					<span
						v-else-if="status[file.file_id] === 'importing'"
						class="flex items-center gap-1 text-sm text-secondary"
					>
						<SpinnerIcon class="animate-spin" /> {{ formatMessage(messages.importing) }}
					</span>
					<Button v-else @click="openPage(file)">
						<ExternalIcon /> {{ formatMessage(messages.download) }}
					</Button>
				</article>
			</div>
			<p v-if="allDone" class="m-0 font-semibold text-green">
				{{ formatMessage(messages.allDone) }}
			</p>
		</div>
		<template #actions>
			<div class="flex justify-end gap-2">
				<Button type="colored" :color="allDone ? 'brand' : undefined" @click="modal?.hide()">
					{{ allDone ? formatMessage(messages.close) : formatMessage(messages.skip) }}
				</Button>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
// Terrarium: файли CurseForge, які автор заборонив качати стороннім лаунчерам. Лаунчер
// їх не тягне з CDN — гравець качає сам на сайті CurseForge, а вікно стежить за
// «Завантаженнями», упізнає файл за sha1 і кладе його в примірник.
import { CheckIcon, ExternalIcon, SpinnerIcon } from '@modrinth/assets'
import { Button, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, onBeforeUnmount, ref, watch } from 'vue'

import {
	type CurseForgeBlockedFile,
	terrarium_curseforge_downloads_dir,
	terrarium_curseforge_import_blocked,
	terrarium_curseforge_scan_downloads,
} from '@/helpers/curseforge'
import { curseforgeBlockedRequest } from '@/helpers/curseforge-blocked'
import { wait_for_install_job } from '@/helpers/install'
import { injectAppEvents } from '@/providers/app-events'

const { formatMessage } = useVIntl()
const appEvents = injectAppEvents()

const messages = defineMessages({
	header: {
		id: 'terrarium.cf-blocked.header',
		defaultMessage: 'Ці файли треба завантажити вручну',
	},
	hint: {
		id: 'terrarium.cf-blocked.hint',
		defaultMessage:
			'Автори цих модів заборонили завантаження через сторонні лаунчери, а на Modrinth їх немає. Натисни «Завантажити» — відкриється сторінка CurseForge у браузері. Щойно файл з’явиться в «Завантаженнях», лаунчер сам його знайде й поставить.',
	},
	watching: { id: 'terrarium.cf-blocked.watching', defaultMessage: 'Стежу за текою: {dir}' },
	noDownloadsDir: {
		id: 'terrarium.cf-blocked.no-downloads-dir',
		defaultMessage: 'Не вдалося знайти теку «Завантаження» — постав ці файли в примірник вручну.',
	},
	download: { id: 'terrarium.cf-blocked.download', defaultMessage: 'Завантажити' },
	importing: { id: 'terrarium.cf-blocked.importing', defaultMessage: 'Ставлю…' },
	done: { id: 'terrarium.cf-blocked.done', defaultMessage: 'Встановлено' },
	allDone: { id: 'terrarium.cf-blocked.all-done', defaultMessage: 'Усі файли на місці.' },
	skip: { id: 'terrarium.cf-blocked.skip', defaultMessage: 'Пропустити' },
	close: { id: 'terrarium.cf-blocked.close', defaultMessage: 'Готово' },
})

type Status = 'waiting' | 'importing' | 'done'

const modal = ref<InstanceType<typeof NewModal>>()
const files = ref<CurseForgeBlockedFile[]>([])
const status = ref<Record<number, Status>>({})
const errors = ref<Record<number, string>>({})
const downloadsDir = ref<string | null>(null)
let instanceId = ''
let installFinished: Promise<unknown> = Promise.resolve()
let timer: ReturnType<typeof setInterval> | null = null
let scanning = false

const pending = computed(() => files.value.filter((f) => status.value[f.file_id] === 'waiting'))
const allDone = computed(
	() => files.value.length > 0 && files.value.every((f) => status.value[f.file_id] === 'done'),
)

watch(curseforgeBlockedRequest, (request) => {
	if (!request) return
	curseforgeBlockedRequest.value = null
	instanceId = request.instanceId
	files.value = request.files
	status.value = Object.fromEntries(request.files.map((f) => [f.file_id, 'waiting' as Status]))
	errors.value = {}
	// Установка збірки ще йде — свої файли кладемо лише після неї
	installFinished = request.jobId
		? wait_for_install_job(appEvents, request.jobId).catch(() => undefined)
		: Promise.resolve()
	void terrarium_curseforge_downloads_dir()
		.then((dir) => (downloadsDir.value = dir))
		.catch(() => (downloadsDir.value = null))
	modal.value?.show()
	startWatching()
})

function openPage(file: CurseForgeBlockedFile) {
	void openUrl(file.page_url)
}

function startWatching() {
	stopWatching()
	timer = setInterval(() => void scan(), 2000)
	void scan()
}

function stopWatching() {
	if (timer) clearInterval(timer)
	timer = null
}

async function scan() {
	if (scanning || pending.value.length === 0) {
		if (pending.value.length === 0) stopWatching()
		return
	}
	scanning = true
	try {
		const found = await terrarium_curseforge_scan_downloads(pending.value)
		if (found.length === 0) return
		await installFinished
		for (const match of found) {
			const file = files.value.find((f) => f.file_id === match.file_id)
			if (!file || status.value[file.file_id] !== 'waiting') continue
			status.value[file.file_id] = 'importing'
			try {
				await terrarium_curseforge_import_blocked(instanceId, file, match.path)
				status.value[file.file_id] = 'done'
				errors.value = Object.fromEntries(
					Object.entries(errors.value).filter(([id]) => Number(id) !== file.file_id),
				)
			} catch (err) {
				status.value[file.file_id] = 'waiting'
				errors.value[file.file_id] = String(err)
			}
		}
	} catch (err) {
		console.warn('CurseForge downloads scan failed', err)
	} finally {
		scanning = false
	}
}

function onHide() {
	stopWatching()
}

onBeforeUnmount(stopWatching)
</script>
