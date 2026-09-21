<template>
	<NewModal ref="modal" :header="formatMessage(messages.header)" max-width="640px">
		<div class="flex flex-col gap-3">
			<div class="flex items-center gap-3">
				<Avatar :src="iconUrl" :alt="modName" size="2.5rem" no-shadow class="shrink-0" />
				<div class="min-w-0">
					<p class="m-0 truncate font-semibold text-contrast">{{ modName }}</p>
					<p class="m-0 flex items-center gap-1 text-sm text-secondary">
						<CurseForgeIcon class="size-4" aria-hidden="true" />
						CurseForge · {{ gameVersion }}<template v-if="loaderLabel"> · {{ loaderLabel }}</template>
					</p>
				</div>
			</div>

			<div v-if="loading" class="flex items-center gap-2 py-6 text-secondary">
				<SpinnerIcon class="animate-spin" />
				{{ formatMessage(messages.loading) }}
			</div>
			<p v-else-if="error" class="m-0 text-red">{{ error }}</p>
			<p v-else-if="files.length === 0" class="m-0 text-secondary">
				{{ formatMessage(messages.empty) }}
			</p>
			<div v-else class="flex max-h-[50vh] flex-col gap-1 overflow-y-auto pr-1">
				<button
					v-for="file in files"
					:key="file.id"
					type="button"
					class="flex w-full items-center gap-3 rounded-xl border border-solid p-3 text-left transition-colors"
					:class="[
						selectedId === file.id
							? 'border-brand bg-brand-highlight'
							: 'border-surface-5 bg-button-bg hover:bg-surface-5',
						!file.downloadable ? 'opacity-70' : '',
					]"
					@click="selectedId = file.id"
				>
					<span
						class="flex size-5 shrink-0 items-center justify-center rounded-full border-2 border-solid"
						:class="selectedId === file.id ? 'border-brand' : 'border-secondary'"
					>
						<span v-if="selectedId === file.id" class="size-2.5 rounded-full bg-brand" />
					</span>
					<span class="flex min-w-0 flex-1 flex-col">
						<span class="flex flex-wrap items-center gap-2">
							<span class="truncate font-semibold text-contrast">{{ file.display_name }}</span>
							<span
								class="rounded-full px-2 py-0.5 text-xs font-semibold"
								:class="releaseClass(file.release_type)"
							>
								{{ releaseLabel(file.release_type) }}
							</span>
							<span
								v-if="file.id === currentFileId"
								class="rounded-full bg-highlight-blue px-2 py-0.5 text-xs font-semibold text-brand-blue"
							>
								{{ formatMessage(messages.installed) }}
							</span>
						</span>
						<span class="truncate text-sm text-secondary">
							{{ file.file_name }} · {{ formatDate(file.file_date) }}
						</span>
						<span v-if="!file.downloadable" class="text-xs text-orange">
							{{ formatMessage(messages.notDownloadable) }}
						</span>
					</span>
					<a
						:href="file.url"
						target="_blank"
						class="shrink-0 text-secondary hover:text-contrast"
						:aria-label="formatMessage(messages.openPage)"
						@click.stop
					>
						<ExternalIcon class="size-4" />
					</a>
				</button>
			</div>
			<div
				v-if="selected && !loading && !error"
				class="rounded-xl border border-solid border-surface-5 bg-button-bg p-3"
			>
				<p class="m-0 mb-1 text-xs font-bold uppercase tracking-wide text-secondary">
					{{ formatMessage(messages.changelog) }}
				</p>
				<div v-if="changelogLoading" class="flex items-center gap-2 text-sm text-secondary">
					<SpinnerIcon class="size-4 animate-spin" />
					{{ formatMessage(messages.changelogLoading) }}
				</div>
				<div
					v-else-if="changelogHtml"
					class="markdown-body max-h-[30vh] overflow-y-auto text-sm"
					v-html="changelogHtml"
				/>
				<p v-else class="m-0 text-sm text-secondary">{{ formatMessage(messages.noChangelog) }}</p>
			</div>
		</div>

		<template #actions>
			<div class="flex justify-end gap-2">
				<Button type="outlined" @click="modal?.hide()">
					<XIcon />
					{{ formatMessage(commonMessages.cancelButton) }}
				</Button>
				<Button
					v-if="selected && !selected.downloadable"
					type="colored"
					color="brand"
					@click="openSelectedPage"
				>
					<ExternalIcon />
					{{ formatMessage(messages.openPage) }}
				</Button>
				<Button
					v-else
					type="colored"
					color="brand"
					:disabled="!selected || selected.id === currentFileId || busy"
					@click="confirm"
				>
					<SpinnerIcon v-if="busy" class="animate-spin" />
					<ArrowLeftRightIcon v-else />
					{{ formatMessage(messages.switch) }}
				</Button>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import {
	ArrowLeftRightIcon,
	CurseForgeIcon,
	ExternalIcon,
	SpinnerIcon,
	XIcon,
} from '@modrinth/assets'
import { Avatar, Button, commonMessages, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { renderString } from '@modrinth/utils'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, ref, watch } from 'vue'

import { terrarium_curseforge_get_changelog } from '@/helpers/curseforge'
import { type CurseForgeFile, terrarium_curseforge_list_files } from '@/helpers/terrarium'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	header: { id: 'terrarium.curseforge.versions.header', defaultMessage: 'Змінити версію' },
	loading: {
		id: 'terrarium.curseforge.versions.loading',
		defaultMessage: 'Завантажую список файлів з CurseForge…',
	},
	empty: {
		id: 'terrarium.curseforge.versions.empty',
		defaultMessage: 'На CurseForge немає файлів для цієї версії гри та завантажувача.',
	},
	installed: { id: 'terrarium.curseforge.versions.installed', defaultMessage: 'Встановлено' },
	notDownloadable: {
		id: 'terrarium.curseforge.versions.not-downloadable',
		defaultMessage: 'Автор заборонив сторонні завантаження — лише зі сторінки CurseForge',
	},
	openPage: { id: 'terrarium.curseforge.versions.open', defaultMessage: 'Відкрити на CurseForge' },
	switch: { id: 'terrarium.curseforge.versions.switch', defaultMessage: 'Змінити версію' },
	release: { id: 'terrarium.curseforge.versions.release', defaultMessage: 'Реліз' },
	beta: { id: 'terrarium.curseforge.versions.beta', defaultMessage: 'Бета' },
	alpha: { id: 'terrarium.curseforge.versions.alpha', defaultMessage: 'Альфа' },
	changelog: { id: 'terrarium.curseforge.versions.changelog', defaultMessage: 'Список змін' },
	changelogLoading: {
		id: 'terrarium.curseforge.versions.changelog-loading',
		defaultMessage: 'Завантажую список змін…',
	},
	noChangelog: {
		id: 'terrarium.curseforge.versions.no-changelog',
		defaultMessage: 'Автор не додав список змін до цього файлу.',
	},
})

const modal = ref<InstanceType<typeof NewModal>>()
const modName = ref('')
const iconUrl = ref<string | null>(null)
const gameVersion = ref('')
const loaderLabel = ref('')
const currentFileId = ref<number | null>(null)
const files = ref<CurseForgeFile[]>([])
const selectedId = ref<number | null>(null)
const loading = ref(false)
const busy = ref(false)
const error = ref<string | null>(null)
let onSelect: ((file: CurseForgeFile) => Promise<void>) | null = null
let requestId = 0

const selected = computed(() => files.value.find((f) => f.id === selectedId.value) ?? null)

// Чейнджлог вибраного файлу — лениво, з кешем бекенду
const changelogHtml = ref('')
const changelogLoading = ref(false)
let currentModId = 0
watch(selected, async (file) => {
	changelogHtml.value = ''
	if (!file) return
	changelogLoading.value = true
	const wanted = file.id
	try {
		const raw = await terrarium_curseforge_get_changelog(currentModId, file.id)
		if (selected.value?.id !== wanted) return
		changelogHtml.value = raw.trim() ? renderString(raw) : ''
	} catch {
		if (selected.value?.id === wanted) changelogHtml.value = ''
	} finally {
		if (selected.value?.id === wanted) changelogLoading.value = false
	}
})

const loaderNames: Record<string, string> = {
	forge: 'Forge',
	fabric: 'Fabric',
	quilt: 'Quilt',
	neoforge: 'NeoForge',
	vanilla: 'Vanilla',
}

function releaseLabel(type: number) {
	return formatMessage(type === 3 ? messages.alpha : type === 2 ? messages.beta : messages.release)
}

function releaseClass(type: number) {
	if (type === 3) return 'bg-highlight-red text-red'
	if (type === 2) return 'bg-highlight-orange text-orange'
	return 'bg-highlight-green text-green'
}

function formatDate(value: string) {
	const date = new Date(value)
	return Number.isNaN(date.getTime()) ? value : date.toLocaleDateString()
}

async function show(options: {
	modId: number
	slug: string
	modName: string
	iconUrl?: string | null
	gameVersion: string
	/** null — ресурспак/шейдер/датапак: завантажувач не має значення */
	loader: string | null
	currentFileId: number
	preselectFileId?: number
	onSelect: (file: CurseForgeFile) => Promise<void>
}) {
	currentModId = options.modId
	modName.value = options.modName
	iconUrl.value = options.iconUrl ?? null
	gameVersion.value = options.gameVersion
	loaderLabel.value = options.loader ? (loaderNames[options.loader] ?? options.loader) : ''
	currentFileId.value = options.currentFileId
	selectedId.value = options.preselectFileId ?? options.currentFileId
	files.value = []
	error.value = null
	busy.value = false
	loading.value = true
	onSelect = options.onSelect
	modal.value?.show()

	const id = ++requestId
	try {
		const list = await terrarium_curseforge_list_files(
			options.modId,
			options.slug,
			options.gameVersion,
			options.loader,
		)
		if (id !== requestId) return
		files.value = list
	} catch (err) {
		if (id !== requestId) return
		error.value = String(err)
	} finally {
		if (id === requestId) loading.value = false
	}
}

async function confirm() {
	const file = selected.value
	if (!file || !onSelect || busy.value) return
	busy.value = true
	try {
		await onSelect(file)
		modal.value?.hide()
	} finally {
		busy.value = false
	}
}

function openSelectedPage() {
	if (selected.value) void openUrl(selected.value.url)
}

defineExpose({ show })
</script>
