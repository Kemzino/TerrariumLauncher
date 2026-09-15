<script setup lang="ts">
import {
	ArrowLeftIcon,
	ArrowRightIcon,
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
	type SyncEntry,
	type SyncPreview,
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
			'Зліва — клієнтська збірка, справа — серверна. Познач групи модів чи конфіги з одного боку і перенеси їх на інший. У цілі нічого зайвого не видаляється — лише замінюються файли з тим самим іменем.',
	},
	client: { id: 'terrarium.sync.client', defaultMessage: 'Клієнт' },
	server: { id: 'terrarium.sync.server', defaultMessage: 'Сервер' },
	groups: { id: 'terrarium.sync.groups', defaultMessage: 'Групи модів' },
	configs: { id: 'terrarium.sync.configs', defaultMessage: 'Конфіги' },
	ungrouped: { id: 'terrarium.sync.ungrouped', defaultMessage: 'Без групи' },
	nothing: { id: 'terrarium.sync.nothing', defaultMessage: 'Порожньо' },
	loading: { id: 'terrarium.sync.loading', defaultMessage: 'Порівнюю збірки…' },
	filesLine: {
		id: 'terrarium.sync.files-line',
		defaultMessage: '{count, plural, one {# файл} few {# файли} other {# файлів}} · {size}',
	},
	newBadge: { id: 'terrarium.sync.new-badge', defaultMessage: '{count} нових' },
	changedBadge: { id: 'terrarium.sync.changed-badge', defaultMessage: '{count} змін.' },
	sameBadge: { id: 'terrarium.sync.same-badge', defaultMessage: 'збігається' },
	selectChanged: { id: 'terrarium.sync.select-changed', defaultMessage: 'Обрати з відмінностями' },
	clear: { id: 'terrarium.sync.clear', defaultMessage: 'Зняти' },
	toServer: { id: 'terrarium.sync.to-server', defaultMessage: 'На сервер ({count})' },
	toClient: { id: 'terrarium.sync.to-client', defaultMessage: 'На клієнт ({count})' },
	applying: { id: 'terrarium.sync.applying', defaultMessage: 'Переношу…' },
	done: {
		id: 'terrarium.sync.done',
		defaultMessage:
			'Перенесено {copied, plural, one {# файл} few {# файли} other {# файлів}}, без змін: {same}',
	},
})

const modal = ref<InstanceType<typeof NewModal>>()
const clientId = ref<string | null>(null)
const serverId = ref<string | null>(null)
const loading = ref(false)
const busy = ref<PackKind | null>(null)

/** Що є з кожного боку (з різницею відносно протилежного). */
const previews = ref<Record<PackKind, SyncPreview | null>>({ client: null, server: null })
const selected = ref<Record<PackKind, Record<string, boolean>>>({ client: {}, server: {} })

const emit = defineEmits<{ synced: [target: PackKind] }>()

const SIDES: PackKind[] = ['client', 'server']
const BLOCKS = ['groups', 'configs'] as const

function idOf(side: PackKind) {
	return side === 'client' ? clientId.value : serverId.value
}
function otherSide(side: PackKind): PackKind {
	return side === 'client' ? 'server' : 'client'
}
function keyOf(entry: SyncEntry) {
	return `${entry.kind}:${entry.key}`
}
function hasDiff(entry: SyncEntry) {
	return entry.new_files > 0 || entry.changed_files > 0
}

function selectedEntries(side: PackKind) {
	const p = previews.value[side]
	if (!p) return { groups: [] as SyncEntry[], configs: [] as SyncEntry[] }
	const chosen = selected.value[side]
	return {
		groups: p.groups.filter((e) => chosen[keyOf(e)]),
		configs: p.configs.filter((e) => chosen[keyOf(e)]),
	}
}
const selectedCount = computed<Record<PackKind, number>>(() => ({
	client: selectedEntries('client').groups.length + selectedEntries('client').configs.length,
	server: selectedEntries('server').groups.length + selectedEntries('server').configs.length,
}))

async function load() {
	const client = clientId.value
	const server = serverId.value
	if (!client || !server) return
	loading.value = true
	previews.value = { client: null, server: null }
	selected.value = { client: {}, server: {} }
	try {
		const [fromClient, fromServer] = await Promise.all([
			terrarium_sync_preview(client, server),
			terrarium_sync_preview(server, client),
		])
		previews.value = { client: fromClient, server: fromServer }
	} catch (err) {
		handleError(err)
	} finally {
		loading.value = false
	}
}

function selectChanged(side: PackKind) {
	const p = previews.value[side]
	if (!p) return
	const next: Record<string, boolean> = {}
	for (const entry of [...p.groups, ...p.configs]) next[keyOf(entry)] = hasDiff(entry)
	selected.value = { ...selected.value, [side]: next }
}

function clearSide(side: PackKind) {
	selected.value = { ...selected.value, [side]: {} }
}

function toggle(side: PackKind, entry: SyncEntry, value?: boolean) {
	const key = keyOf(entry)
	const current = selected.value[side]
	selected.value = {
		...selected.value,
		[side]: { ...current, [key]: value ?? !current[key] },
	}
}

async function apply(from: PackKind) {
	const source = idOf(from)
	const target = idOf(otherSide(from))
	if (!source || !target || selectedCount.value[from] === 0 || busy.value) return
	busy.value = from
	try {
		const chosen = selectedEntries(from)
		const result = await terrarium_sync_apply({
			source_instance_id: source,
			target_instance_id: target,
			groups: chosen.groups.map((e) => e.key),
			configs: chosen.configs.map((e) => e.key),
		})
		addNotification({
			type: 'success',
			title: formatMessage(messages.done, {
				copied: result.copied,
				same: result.skipped_same,
			}),
		})
		emit('synced', otherSide(from))
		await load()
	} catch (err) {
		handleError(err)
	} finally {
		busy.value = null
	}
}

function formatSize(bytes: number) {
	if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(2)} ГБ`
	if (bytes >= 1e6) return `${(bytes / 1e6).toFixed(1)} МБ`
	return `${Math.max(1, Math.round(bytes / 1e3))} КБ`
}

function entryName(entry: SyncEntry) {
	return entry.kind === 'mod_group' && !entry.key ? formatMessage(messages.ungrouped) : entry.name
}

async function show(client: string, server: string) {
	clientId.value = client
	serverId.value = server
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
		width="64rem"
		max-width="calc(100vw - 2rem)"
	>
		<div class="flex flex-col gap-4">
			<p class="m-0 text-secondary">{{ formatMessage(messages.intro) }}</p>

			<div v-if="loading" class="flex items-center gap-2 text-secondary">
				<SpinnerIcon class="animate-spin" /> {{ formatMessage(messages.loading) }}
			</div>

			<div v-else class="sync-columns">
				<section v-for="side in SIDES" :key="side" class="sync-column">
					<header class="sync-column__head">
						<h3 class="sync-column__title">
							{{ formatMessage(side === 'client' ? messages.client : messages.server) }}
						</h3>
						<div class="flex items-center gap-1">
							<Button size="sm" type="transparent" @click="selectChanged(side)">
								{{ formatMessage(messages.selectChanged) }}
							</Button>
							<Button
								v-if="selectedCount[side] > 0"
								size="sm"
								type="transparent"
								@click="clearSide(side)"
							>
								{{ formatMessage(messages.clear) }}
							</Button>
						</div>
					</header>

					<template v-if="previews[side]">
						<div v-for="block in BLOCKS" :key="block" class="sync-block">
							<h4 class="sync-block__title">
								<FolderIcon v-if="block === 'groups'" />
								<SettingsIcon v-else />
								{{ formatMessage(block === 'groups' ? messages.groups : messages.configs) }}
							</h4>
							<p v-if="previews[side]![block].length === 0" class="sync-empty">
								{{ formatMessage(messages.nothing) }}
							</p>
							<div v-else class="sync-list">
								<div
									v-for="entry in previews[side]![block]"
									:key="keyOf(entry)"
									class="sync-item"
									:class="{ 'is-same': !hasDiff(entry) }"
									@click="toggle(side, entry)"
								>
									<Checkbox
										:model-value="!!selected[side][keyOf(entry)]"
										@click.stop
										@update:model-value="toggle(side, entry, $event)"
									/>
									<span class="sync-item__text">
										<span class="sync-item__name" :title="entry.key">{{ entryName(entry) }}</span>
										<span class="sync-item__sub">
											{{
												formatMessage(messages.filesLine, {
													count: entry.files,
													size: formatSize(entry.size),
												})
											}}
										</span>
									</span>
									<span v-if="entry.new_files" class="sync-item__tag is-new">
										{{ formatMessage(messages.newBadge, { count: entry.new_files }) }}
									</span>
									<span v-if="entry.changed_files" class="sync-item__tag is-changed">
										{{ formatMessage(messages.changedBadge, { count: entry.changed_files }) }}
									</span>
									<span v-if="!hasDiff(entry)" class="sync-item__tag">
										{{ formatMessage(messages.sameBadge) }}
									</span>
								</div>
							</div>
						</div>
					</template>
				</section>
			</div>
		</div>
		<template #actions>
			<div class="flex flex-wrap items-center justify-between gap-2">
				<Button type="outlined" :disabled="!!busy" @click="modal?.hide()">
					<XIcon />
					{{ formatMessage(commonMessages.cancelButton) }}
				</Button>
				<div class="flex items-center gap-2">
					<Button
						type="colored"
						color="brand"
						:disabled="!!busy || loading || selectedCount.server === 0"
						@click="apply('server')"
					>
						<SpinnerIcon v-if="busy === 'server'" class="animate-spin" />
						<ArrowLeftIcon v-else />
						{{
							busy === 'server'
								? formatMessage(messages.applying)
								: formatMessage(messages.toClient, { count: selectedCount.server })
						}}
					</Button>
					<Button
						type="colored"
						color="brand"
						:disabled="!!busy || loading || selectedCount.client === 0"
						@click="apply('client')"
					>
						<SpinnerIcon v-if="busy === 'client'" class="animate-spin" />
						<ArrowRightIcon v-else />
						{{
							busy === 'client'
								? formatMessage(messages.applying)
								: formatMessage(messages.toServer, { count: selectedCount.client })
						}}
					</Button>
				</div>
			</div>
		</template>
	</NewModal>
</template>

<style scoped lang="scss">
.sync-columns {
	display: grid;
	grid-template-columns: 1fr 1fr;
	gap: 1rem;

	@media (max-width: 800px) {
		grid-template-columns: 1fr;
	}
}

.sync-column {
	display: flex;
	flex-direction: column;
	gap: 0.75rem;
	min-width: 0;
	border: 1px solid var(--color-divider);
	border-radius: var(--radius-lg);
	padding: 0.75rem;
}

.sync-column__head {
	display: flex;
	align-items: center;
	justify-content: space-between;
	gap: 0.5rem;
	flex-wrap: wrap;
}

.sync-column__title {
	margin: 0;
	font-size: 1.1rem;
	font-weight: 700;
	color: var(--color-contrast);
}

.sync-block__title {
	display: flex;
	align-items: center;
	gap: 0.4rem;
	margin: 0 0 0.35rem;
	font-size: 0.85rem;
	font-weight: 600;
	letter-spacing: 0.04em;
	text-transform: uppercase;
	color: var(--color-secondary);

	svg {
		width: 1rem;
		height: 1rem;
	}
}

.sync-empty {
	margin: 0;
	padding: 0.4rem 0.5rem;
	font-size: 0.85rem;
	color: var(--color-secondary);
}

.sync-list {
	display: flex;
	flex-direction: column;
	gap: 0.15rem;
}

.sync-item {
	display: flex;
	align-items: center;
	gap: 0.6rem;
	padding: 0.35rem 0.5rem;
	border-radius: var(--radius-md);
	cursor: pointer;

	&:hover {
		background: var(--color-button-bg);
	}

	&.is-same {
		opacity: 0.65;
	}
}

.sync-item__text {
	display: flex;
	flex: 1;
	min-width: 0;
	flex-direction: column;
}

.sync-item__name {
	font-weight: 600;
	color: var(--color-contrast);
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.sync-item__sub {
	font-size: 0.78rem;
	color: var(--color-secondary);
}

.sync-item__tag {
	flex-shrink: 0;
	padding: 0.05rem 0.45rem;
	border-radius: 9999px;
	background: var(--color-button-bg);
	color: var(--color-secondary);
	font-size: 0.72rem;
	font-weight: 600;
	white-space: nowrap;

	&.is-new {
		background: var(--color-brand-highlight);
		color: var(--color-brand);
	}

	&.is-changed {
		background: color-mix(in srgb, var(--color-orange) 18%, transparent);
		color: var(--color-orange);
	}
}
</style>
