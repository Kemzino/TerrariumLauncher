<script setup lang="ts">
import {
	ArrowLeftIcon,
	ChevronDownIcon,
	FolderIcon,
	SettingsIcon,
	SpinnerIcon,
	XIcon,
} from '@modrinth/assets'
import {
	Button,
	Checkbox,
	commonMessages,
	defineMessages,
	injectNotificationManager,
	NewModal,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import {
	type PackKind,
	type SyncFile,
	type SyncPreview,
	type SyncSection,
	terrarium_sync_apply,
	terrarium_sync_preview,
} from '@/helpers/terrarium'

const { formatMessage } = useVIntl()
const { handleError, addNotification } = injectNotificationManager()

const messages = defineMessages({
	header: { id: 'terrarium.sync.header', defaultMessage: 'Синхронізація збірок' },
	intro: {
		id: 'terrarium.sync.intro',
		defaultMessage:
			'Кожен рядок — файл: зліва як він у клієнтській збірці, справа — у серверній. Познач файли з того боку, звідки їх узяти, і перенеси на інший. У цілі нічого не видаляється — лише замінюються файли з тим самим іменем.',
	},
	client: { id: 'terrarium.sync.client', defaultMessage: 'Клієнт' },
	server: { id: 'terrarium.sync.server', defaultMessage: 'Сервер' },
	file: { id: 'terrarium.sync.file', defaultMessage: 'Файл' },
	mods: { id: 'terrarium.sync.mods', defaultMessage: 'Моди' },
	configs: { id: 'terrarium.sync.configs', defaultMessage: 'Конфіги' },
	ungrouped: { id: 'terrarium.sync.ungrouped', defaultMessage: 'Без групи' },
	missing: { id: 'terrarium.sync.missing', defaultMessage: 'немає' },
	loading: { id: 'terrarium.sync.loading', defaultMessage: 'Порівнюю збірки…' },
	onlyDiff: { id: 'terrarium.sync.only-diff', defaultMessage: 'Лише відмінності' },
	diffCount: {
		id: 'terrarium.sync.diff-count',
		defaultMessage:
			'{count, plural, =0 {без відмінностей} one {# відмінність} few {# відмінності} other {# відмінностей}}',
	},
	sectionAllToServer: { id: 'terrarium.sync.section-to-server', defaultMessage: 'усе →' },
	sectionAllToClient: { id: 'terrarium.sync.section-to-client', defaultMessage: '← усе' },
	statusSame: { id: 'terrarium.sync.status-same', defaultMessage: 'однакові' },
	statusDiffers: { id: 'terrarium.sync.status-differs', defaultMessage: 'відрізняються' },
	statusClientOnly: { id: 'terrarium.sync.status-client-only', defaultMessage: 'лише клієнт' },
	statusServerOnly: { id: 'terrarium.sync.status-server-only', defaultMessage: 'лише сервер' },
	inGroup: { id: 'terrarium.sync.in-group', defaultMessage: 'у групі «{group}»' },
	toServer: { id: 'terrarium.sync.to-server', defaultMessage: 'На сервер ({count})' },
	toClient: { id: 'terrarium.sync.to-client', defaultMessage: 'На клієнт ({count})' },
	apply: { id: 'terrarium.sync.apply', defaultMessage: 'Перенести' },
	applying: { id: 'terrarium.sync.applying', defaultMessage: 'Переношу…' },
	done: {
		id: 'terrarium.sync.done',
		defaultMessage: 'Перенесено {copied, plural, one {# файл} few {# файли} other {# файлів}}',
	},
	nothing: { id: 'terrarium.sync.nothing', defaultMessage: 'В обох збірках порожньо' },
})

const modal = ref<InstanceType<typeof NewModal>>()
const clientId = ref<string | null>(null)
const serverId = ref<string | null>(null)
const loading = ref(false)
const busy = ref(false)
const preview = ref<SyncPreview | null>(null)
const onlyDiff = ref(true)
const collapsed = ref<Record<string, boolean>>({})
/** Обрані файли: з клієнта на сервер / із сервера на клієнт (ключі SyncFile.key). */
const toServer = ref<Record<string, boolean>>({})
const toClient = ref<Record<string, boolean>>({})

const emit = defineEmits<{ synced: [target: PackKind] }>()

function sectionKey(section: SyncSection) {
	return `${section.kind}:${section.key}`
}
function isDiff(file: SyncFile) {
	return file.status !== 'same'
}
function sectionTitle(section: SyncSection) {
	if (section.kind === 'mod') {
		return section.key ? section.name : formatMessage(messages.ungrouped)
	}
	return section.name
}
function diffCount(section: SyncSection) {
	return section.files.filter(isDiff).length
}

const visibleSections = computed(() => {
	if (!preview.value) return []
	return preview.value.sections
		.map((section) => ({
			section,
			files: onlyDiff.value ? section.files.filter(isDiff) : section.files,
		}))
		.filter((s) => s.files.length > 0)
})
const modSections = computed(() => visibleSections.value.filter((s) => s.section.kind === 'mod'))
const configSections = computed(() =>
	visibleSections.value.filter((s) => s.section.kind === 'config'),
)

const toServerCount = computed(() => Object.values(toServer.value).filter(Boolean).length)
const toClientCount = computed(() => Object.values(toClient.value).filter(Boolean).length)

function isCollapsed(section: SyncSection) {
	const key = sectionKey(section)
	// Секції без відмінностей згорнуті за замовчуванням
	return collapsed.value[key] ?? diffCount(section) === 0
}
function toggleCollapsed(section: SyncSection) {
	const key = sectionKey(section)
	collapsed.value = { ...collapsed.value, [key]: !isCollapsed(section) }
}

/** Файл можна взяти з боку, де він є; вибір однієї сторони знімає іншу. */
function pickToServer(file: SyncFile, value: boolean) {
	if (file.client_size === null) return
	toServer.value = { ...toServer.value, [file.key]: value }
	if (value && toClient.value[file.key]) {
		toClient.value = { ...toClient.value, [file.key]: false }
	}
}
function pickToClient(file: SyncFile, value: boolean) {
	if (file.server_size === null) return
	toClient.value = { ...toClient.value, [file.key]: value }
	if (value && toServer.value[file.key]) {
		toServer.value = { ...toServer.value, [file.key]: false }
	}
}
function sectionAll(files: SyncFile[], direction: PackKind) {
	for (const file of files) {
		if (!isDiff(file)) continue
		if (direction === 'server') pickToServer(file, true)
		else pickToClient(file, true)
	}
}

async function load() {
	if (!clientId.value || !serverId.value) return
	loading.value = true
	preview.value = null
	toServer.value = {}
	toClient.value = {}
	try {
		preview.value = await terrarium_sync_preview(clientId.value, serverId.value)
	} catch (err) {
		handleError(err)
	} finally {
		loading.value = false
	}
}

async function apply() {
	if (!clientId.value || !serverId.value || busy.value) return
	const server = Object.entries(toServer.value)
		.filter(([, v]) => v)
		.map(([k]) => k)
	const client = Object.entries(toClient.value)
		.filter(([, v]) => v)
		.map(([k]) => k)
	if (server.length === 0 && client.length === 0) return
	busy.value = true
	try {
		const result = await terrarium_sync_apply({
			client_instance_id: clientId.value,
			server_instance_id: serverId.value,
			to_server: server,
			to_client: client,
		})
		addNotification({
			type: 'success',
			title: formatMessage(messages.done, { copied: result.copied }),
		})
		if (server.length) emit('synced', 'server')
		if (client.length) emit('synced', 'client')
		await load()
	} catch (err) {
		handleError(err)
	} finally {
		busy.value = false
	}
}

function formatSize(bytes: number | null) {
	if (bytes === null) return formatMessage(messages.missing)
	if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(2)} ГБ`
	if (bytes >= 1e6) return `${(bytes / 1e6).toFixed(1)} МБ`
	return `${Math.max(1, Math.round(bytes / 1e3))} КБ`
}

function statusLabel(file: SyncFile) {
	switch (file.status) {
		case 'same':
			return formatMessage(messages.statusSame)
		case 'differs':
			return formatMessage(messages.statusDiffers)
		case 'client_only':
			return formatMessage(messages.statusClientOnly)
		default:
			return formatMessage(messages.statusServerOnly)
	}
}

/** Підпис комірки: розмір і, якщо мод лежить в іншій групі, ніж секція, — де саме. */
function cellNote(file: SyncFile, side: PackKind, section: SyncSection) {
	const group = side === 'client' ? file.client_group : file.server_group
	if (file.kind !== 'mod' || group === null || group === section.key) return null
	return formatMessage(messages.inGroup, { group })
}

async function show(client: string, server: string) {
	clientId.value = client
	serverId.value = server
	collapsed.value = {}
	modal.value?.show()
	await load()
}

defineExpose({ show })
</script>

<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.header)"
		scrollable
		width="72rem"
		max-width="calc(100vw - 2rem)"
	>
		<div class="flex flex-col gap-4">
			<p class="m-0 text-secondary">{{ formatMessage(messages.intro) }}</p>

			<div v-if="loading" class="flex items-center gap-2 text-secondary">
				<SpinnerIcon class="animate-spin" /> {{ formatMessage(messages.loading) }}
			</div>

			<template v-else-if="preview">
				<div class="flex flex-wrap items-center justify-between gap-2">
					<Checkbox v-model="onlyDiff" :label="formatMessage(messages.onlyDiff)" />
				</div>

				<p v-if="visibleSections.length === 0" class="m-0 text-secondary">
					{{ formatMessage(messages.nothing) }}
				</p>

				<div v-else class="sync-table">
					<div class="sync-row sync-row--head">
						<span class="sync-cell sync-cell--side sync-cell--client">
							{{ formatMessage(messages.client) }}
						</span>
						<span class="sync-cell sync-cell--name">{{ formatMessage(messages.file) }}</span>
						<span class="sync-cell sync-cell--side sync-cell--server">
							{{ formatMessage(messages.server) }}
						</span>
					</div>

					<template
						v-for="group in [modSections, configSections]"
						:key="group === modSections ? 'mods' : 'configs'"
					>
						<div v-if="group.length" class="sync-kind">
							<FolderIcon v-if="group === modSections" />
							<SettingsIcon v-else />
							{{ formatMessage(group === modSections ? messages.mods : messages.configs) }}
						</div>
						<template v-for="{ section, files } in group" :key="sectionKey(section)">
							<div class="sync-section">
								<button
									type="button"
									class="sync-section__toggle"
									:aria-expanded="!isCollapsed(section)"
									@click="toggleCollapsed(section)"
								>
									<ChevronDownIcon
										class="sync-section__chevron"
										:class="{ 'is-collapsed': isCollapsed(section) }"
									/>
									<span class="sync-section__name">{{ sectionTitle(section) }}</span>
									<span class="sync-section__count">
										{{ formatMessage(messages.diffCount, { count: diffCount(section) }) }}
									</span>
								</button>
								<span v-if="diffCount(section) > 0" class="sync-section__actions">
									<button type="button" @click="sectionAll(section.files, 'client')">
										{{ formatMessage(messages.sectionAllToClient) }}
									</button>
									<button type="button" @click="sectionAll(section.files, 'server')">
										{{ formatMessage(messages.sectionAllToServer) }}
									</button>
								</span>
							</div>
							<template v-if="!isCollapsed(section)">
								<div
									v-for="file in files"
									:key="file.key"
									class="sync-row"
									:class="`is-${file.status}`"
								>
									<span class="sync-cell sync-cell--side sync-cell--client">
										<Checkbox
											v-if="file.client_size !== null"
											:model-value="!!toServer[file.key]"
											:disabled="file.status === 'same'"
											@update:model-value="pickToServer(file, $event)"
										/>
										<span
											class="sync-cell__meta"
											:class="{ 'is-missing': file.client_size === null }"
										>
											{{ formatSize(file.client_size) }}
											<small v-if="cellNote(file, 'client', section)">{{
												cellNote(file, 'client', section)
											}}</small>
										</span>
									</span>
									<span class="sync-cell sync-cell--name">
										<span class="sync-cell__name" :title="file.key">{{ file.name }}</span>
										<span class="sync-cell__status">{{ statusLabel(file) }}</span>
									</span>
									<span class="sync-cell sync-cell--side sync-cell--server">
										<span
											class="sync-cell__meta"
											:class="{ 'is-missing': file.server_size === null }"
										>
											{{ formatSize(file.server_size) }}
											<small v-if="cellNote(file, 'server', section)">{{
												cellNote(file, 'server', section)
											}}</small>
										</span>
										<Checkbox
											v-if="file.server_size !== null"
											:model-value="!!toClient[file.key]"
											:disabled="file.status === 'same'"
											@update:model-value="pickToClient(file, $event)"
										/>
									</span>
								</div>
							</template>
						</template>
					</template>
				</div>
			</template>
		</div>
		<template #actions>
			<div class="flex flex-wrap items-center justify-between gap-2">
				<Button type="outlined" :disabled="busy" @click="modal?.hide()">
					<XIcon />
					{{ formatMessage(commonMessages.cancelButton) }}
				</Button>
				<div class="flex items-center gap-2">
					<span v-if="toClientCount" class="sync-pill">
						<ArrowLeftIcon /> {{ formatMessage(messages.toClient, { count: toClientCount }) }}
					</span>
					<span v-if="toServerCount" class="sync-pill">
						{{ formatMessage(messages.toServer, { count: toServerCount }) }}
						<ArrowLeftIcon class="rotate-180" />
					</span>
					<Button
						type="colored"
						color="brand"
						:disabled="busy || loading || (toServerCount === 0 && toClientCount === 0)"
						@click="apply"
					>
						<SpinnerIcon v-if="busy" class="animate-spin" />
						{{ busy ? formatMessage(messages.applying) : formatMessage(messages.apply) }}
					</Button>
				</div>
			</div>
		</template>
	</NewModal>
</template>

<style scoped lang="scss">
.sync-table {
	display: flex;
	flex-direction: column;
	border: 1px solid var(--color-divider);
	border-radius: var(--radius-lg);
	overflow: hidden;
}

.sync-row {
	display: grid;
	grid-template-columns: minmax(9rem, 1fr) minmax(0, 2fr) minmax(9rem, 1fr);
	align-items: center;
	border-top: 1px solid var(--color-divider);

	&--head {
		border-top: 0;
		background: var(--color-bg);
		font-weight: 700;
		color: var(--color-contrast);
	}

	&.is-client_only .sync-cell--server,
	&.is-server_only .sync-cell--client {
		opacity: 0.5;
	}

	&.is-same {
		opacity: 0.6;
	}
}

.sync-cell {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	min-width: 0;
	padding: 0.4rem 0.75rem;

	&--client {
		justify-content: flex-start;
		border-right: 1px solid var(--color-divider);
	}

	&--server {
		justify-content: flex-end;
		border-left: 1px solid var(--color-divider);
	}

	&--name {
		flex-direction: column;
		align-items: center;
		gap: 0.05rem;
		text-align: center;
	}
}

.sync-cell__name {
	max-width: 100%;
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
	font-weight: 600;
	color: var(--color-contrast);
}

.sync-cell__status {
	font-size: 0.72rem;
	color: var(--color-secondary);

	.is-differs & {
		color: var(--color-orange);
	}
	.is-client_only &,
	.is-server_only & {
		color: var(--color-brand);
	}
}

.sync-cell__meta {
	display: flex;
	flex-direction: column;
	font-size: 0.85rem;
	color: var(--color-base);

	small {
		font-size: 0.7rem;
		color: var(--color-secondary);
	}

	&.is-missing {
		color: var(--color-secondary);
		font-style: italic;
	}
}

.sync-kind {
	display: flex;
	align-items: center;
	gap: 0.4rem;
	padding: 0.5rem 0.75rem 0.25rem;
	border-top: 1px solid var(--color-divider);
	font-size: 0.8rem;
	font-weight: 700;
	letter-spacing: 0.05em;
	text-transform: uppercase;
	color: var(--color-secondary);

	svg {
		width: 1rem;
		height: 1rem;
	}
}

.sync-section {
	display: flex;
	align-items: center;
	justify-content: space-between;
	gap: 0.5rem;
	padding: 0.3rem 0.75rem;
	background: var(--color-button-bg);
	border-top: 1px solid var(--color-divider);
}

.sync-section__toggle {
	display: flex;
	flex: 1;
	min-width: 0;
	align-items: center;
	gap: 0.4rem;
	padding: 0;
	border: 0;
	background: none;
	color: var(--color-contrast);
	font: inherit;
	text-align: left;
	cursor: pointer;
}

.sync-section__chevron {
	width: 1.1rem;
	height: 1.1rem;
	transition: transform 0.12s ease;

	&.is-collapsed {
		transform: rotate(-90deg);
	}
}

.sync-section__name {
	font-weight: 700;
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.sync-section__count {
	font-size: 0.78rem;
	color: var(--color-secondary);
}

.sync-section__actions {
	display: flex;
	gap: 0.35rem;

	button {
		padding: 0.1rem 0.5rem;
		border: 1px solid var(--color-button-border);
		border-radius: 9999px;
		background: transparent;
		color: var(--color-secondary);
		font: inherit;
		font-size: 0.75rem;
		cursor: pointer;

		&:hover {
			color: var(--color-contrast);
			border-color: var(--color-brand);
		}
	}
}

.sync-pill {
	display: inline-flex;
	align-items: center;
	gap: 0.3rem;
	padding: 0.2rem 0.6rem;
	border-radius: 9999px;
	background: var(--color-brand-highlight);
	color: var(--color-brand);
	font-size: 0.85rem;
	font-weight: 600;

	svg {
		width: 1rem;
		height: 1rem;
	}
}
</style>
