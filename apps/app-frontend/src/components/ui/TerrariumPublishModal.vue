<script setup lang="ts">
import {
	ChevronDownIcon,
	FileIcon,
	GlobeIcon,
	RocketIcon,
	SettingsIcon,
	SpinnerIcon,
	TestIcon,
	XIcon,
} from '@modrinth/assets'
import {
	Avatar,
	Button,
	Checkbox,
	commonMessages,
	defineMessages,
	injectNotificationManager,
	Input,
	NewModal,
	Textarea,
	useVIntl,
} from '@modrinth/ui'
import { useQuery } from '@tanstack/vue-query'
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'

import {
	type PackKind,
	type PublishCandidate,
	type PublishedRelease,
	type PublishPreview,
	suggestNextTag,
	terrarium_publish_preview,
	terrarium_publish_release,
} from '@/helpers/terrarium'
import { instanceContentQueryOptions } from '@/pages/instance/query-options'

const { formatMessage } = useVIntl()
const { handleError, addNotification } = injectNotificationManager()
const router = useRouter()

const emit = defineEmits<{ published: [release: PublishedRelease] }>()

const messages = defineMessages({
	header: { id: 'terrarium.publish.header', defaultMessage: 'Опублікувати оновлення збірки' },
	headerServer: {
		id: 'terrarium.publish.header-server',
		defaultMessage: 'Опублікувати оновлення серверної збірки',
	},
	intro: {
		id: 'terrarium.publish.intro',
		defaultMessage:
			'Показано лише те, що відрізняється від останнього релізу. Особисте лиши без галочки — лаунчер запам’ятає і більше не пропонуватиме.',
	},
	tagLabel: { id: 'terrarium.publish.tag', defaultMessage: 'Версія (тег)' },
	nameLabel: { id: 'terrarium.publish.name', defaultMessage: 'Назва релізу' },
	notesLabel: { id: 'terrarium.publish.notes', defaultMessage: 'Що змінилося' },
	notesPlaceholder: {
		id: 'terrarium.publish.notes-placeholder',
		defaultMessage: 'Додано Create Addons, оновлено Sodium…',
	},
	publish: { id: 'terrarium.publish.button', defaultMessage: 'Опублікувати' },
	publishing: { id: 'terrarium.publish.publishing', defaultMessage: 'Пакую й завантажую…' },
	loading: { id: 'terrarium.publish.loading', defaultMessage: 'Порівнюю з останнім релізом…' },
	success: { id: 'terrarium.publish.success', defaultMessage: 'Реліз {tag} опубліковано для всіх' },
	successTest: {
		id: 'terrarium.publish.success-test',
		defaultMessage: 'Тестову версію {tag} опубліковано — бачать адміни й тестери',
	},
	targetLabel: { id: 'terrarium.publish.target', defaultMessage: 'Куди' },
	targetTest: { id: 'terrarium.publish.target-test', defaultMessage: 'У тест' },
	targetTestHint: {
		id: 'terrarium.publish.target-test-hint',
		defaultMessage: 'Бачать лише адміни й тестери; потім «Поширити для всіх» на головній',
	},
	targetAll: { id: 'terrarium.publish.target-all', defaultMessage: 'Усім одразу' },
	targetAllHint: {
		id: 'terrarium.publish.target-all-hint',
		defaultMessage: 'Гравці отримають оновлення відразу, без тестування',
	},
	baseIsTest: {
		id: 'terrarium.publish.base-is-test',
		defaultMessage: 'Порівнюю з тестовою {tag} (ще не поширена для всіх).',
	},
	firstRelease: {
		id: 'terrarium.publish.first-release',
		defaultMessage: 'Релізів ще не було — у пакет піде все, що відмічено.',
	},
	noChanges: {
		id: 'terrarium.publish.no-changes',
		defaultMessage: 'Відмінностей від {tag} немає — реліз буде тим самим.',
	},
	kindNew: { id: 'terrarium.publish.kind-new', defaultMessage: 'нове' },
	kindChanged: { id: 'terrarium.publish.kind-changed', defaultMessage: 'змінено' },
	kindExcluded: { id: 'terrarium.publish.kind-excluded', defaultMessage: 'особисте' },
	kindDisabled: { id: 'terrarium.publish.kind-disabled', defaultMessage: 'вимкнено' },
	unchangedLine: {
		id: 'terrarium.publish.unchanged',
		defaultMessage: 'без змін: {count} — включено як є',
	},
	showUnchanged: { id: 'terrarium.publish.show-unchanged', defaultMessage: 'показати' },
	hideUnchanged: { id: 'terrarium.publish.hide-unchanged', defaultMessage: 'сховати' },
	removedTitle: {
		id: 'terrarium.publish.removed-title',
		defaultMessage: 'Зникли з примірника — випадуть зі збірки',
	},
	summary: {
		id: 'terrarium.publish.summary',
		defaultMessage: 'У реліз піде {included} з {total} файлів · модів {mods}',
	},
	selectAll: { id: 'terrarium.publish.select-all', defaultMessage: 'Усі' },
	selectNone: { id: 'terrarium.publish.select-none', defaultMessage: 'Жодного' },
	brandTitle: { id: 'terrarium.publish.brand', defaultMessage: 'Назва та іконка' },
	brandChange: { id: 'terrarium.publish.brand-change', defaultMessage: 'Змінити' },
	brandWas: { id: 'terrarium.publish.brand-was', defaultMessage: 'у {tag}: {name}' },
	brandIconChanged: { id: 'terrarium.publish.brand-icon', defaultMessage: 'нова іконка' },
	brandHint: {
		id: 'terrarium.publish.brand-hint',
		defaultMessage: 'У гравців примірник перейменується і отримає цю іконку при оновленні.',
	},
	coreTitle: { id: 'terrarium.publish.core', defaultMessage: 'Ядро' },
	coreChange: { id: 'terrarium.publish.core-change', defaultMessage: 'Змінити ядро' },
	coreWas: { id: 'terrarium.publish.core-was', defaultMessage: 'у {tag}: {core}' },
	coreHint: {
		id: 'terrarium.publish.core-hint',
		defaultMessage:
			'Гравці отримають нову версію завантажувача автоматично при оновленні. Сервер — через server_sync.',
	},
	folderMods: { id: 'terrarium.publish.folder-mods', defaultMessage: 'Моди' },
	folderConfig: { id: 'terrarium.publish.folder-config', defaultMessage: 'Конфіги' },
	folderKubejs: { id: 'terrarium.publish.folder-kubejs', defaultMessage: 'KubeJS-скрипти' },
	folderDefaultconfigs: {
		id: 'terrarium.publish.folder-defaultconfigs',
		defaultMessage: 'Стандартні конфіги світів',
	},
	folderResourcepacks: { id: 'terrarium.publish.folder-resourcepacks', defaultMessage: 'Ресурспаки' },
	folderShaderpacks: { id: 'terrarium.publish.folder-shaderpacks', defaultMessage: 'Шейдери' },
	folderDatapacks: { id: 'terrarium.publish.folder-datapacks', defaultMessage: 'Датапаки' },
})

const folderLabels: Record<string, keyof typeof messages> = {
	mods: 'folderMods',
	config: 'folderConfig',
	kubejs: 'folderKubejs',
	defaultconfigs: 'folderDefaultconfigs',
	resourcepacks: 'folderResourcepacks',
	shaderpacks: 'folderShaderpacks',
	datapacks: 'folderDatapacks',
}

const modal = ref<InstanceType<typeof NewModal> | null>(null)
const pack = ref<PackKind>('client')
const instanceId = ref('')
const tag = ref('')
const name = ref('')
const notes = ref('')
// Куди публікуємо: спершу в тест (pre-release; бачать адміни й тестери) або одразу всім
const toTest = ref(true)
const busy = ref(false)
const loading = ref(false)
const preview = ref<PublishPreview | null>(null)
// path → чи включати в реліз
const selected = ref<Record<string, boolean>>({})
const expandedUnchanged = ref<Record<string, boolean>>({})

// Назви й іконки модів беремо з тих самих даних, що й сторінка «Уміст»
const contentQuery = useQuery(
	computed(() => ({
		...instanceContentQueryOptions(instanceId.value),
		enabled: !!instanceId.value,
	})),
)
const modInfo = computed(() => {
	const map = new Map<string, { title: string; icon?: string; version?: string }>()
	for (const item of contentQuery.data.value?.contentItems ?? []) {
		map.set(item.file_name, {
			title: item.project?.title ?? item.file_name,
			icon: item.project?.icon_url ?? undefined,
			version: item.version?.version_number,
		})
	}
	return map
})

function infoFor(c: PublishCandidate) {
	if (c.folder !== 'mods') return null
	return modInfo.value.get(c.file_name.replace(/\.disabled$/, '')) ?? null
}

function subtitleFor(c: PublishCandidate) {
	const info = infoFor(c)
	if (!info) return c.group ? c.group + ' / ' + c.file_name : ''
	const base = info.version ? info.version + ' · ' + c.file_name : c.file_name
	return c.group ? c.group + ' / ' + base : base
}

const iconUrl = ref<string | null>(null)

async function show(
	targetPack: PackKind,
	targetInstanceId: string,
	currentTag: string | null,
	icon: string | null = null,
) {
	pack.value = targetPack
	iconUrl.value = icon
	instanceId.value = targetInstanceId
	toTest.value = true
	tag.value = suggestNextTag(currentTag)
	name.value = `Terrarium ${targetPack === 'server' ? 'Server ' : ''}${tag.value}`
	notes.value = ''
	preview.value = null
	selected.value = {}
	expandedUnchanged.value = {}
	modal.value?.show()

	loading.value = true
	try {
		const result = await terrarium_publish_preview(targetPack, targetInstanceId)
		preview.value = result
		// Наступний тег — після найновішого релізу (включно з тестовим)
		if (result.release_tag) tag.value = suggestNextTag(result.release_tag)
		const first = !result.release_tag
		const initial: Record<string, boolean> = {}
		for (const c of result.candidates) {
			if (c.excluded || c.disabled) {
				initial[c.path] = false
			} else if (c.folder === 'mods') {
				// Новий мод — найімовірніше особистий, тож без галочки (крім першого релізу)
				initial[c.path] = c.in_release || first
			} else {
				// Конфіги та інше: нове і змінене — це і є оновлення, тож із галочкою
				initial[c.path] = true
			}
		}
		selected.value = initial
	} catch (err) {
		handleError(err)
	} finally {
		loading.value = false
	}
}

type Kind = 'new' | 'changed' | 'unchanged' | 'excluded'

function kindOf(c: PublishCandidate, first: boolean): Kind {
	if (c.excluded || c.disabled) return 'excluded'
	if (!c.in_release || first) return 'new'
	if (c.changed) return 'changed'
	return 'unchanged'
}

interface FolderGroup {
	folder: string
	label: string
	attention: PublishCandidate[] // нове + змінене
	unchanged: PublishCandidate[]
	excluded: PublishCandidate[]
}

const folders = computed<FolderGroup[]>(() => {
	const all = preview.value?.candidates ?? []
	const first = !preview.value?.release_tag
	const byFolder = new Map<string, FolderGroup>()
	for (const c of all) {
		let g = byFolder.get(c.folder)
		if (!g) {
			const key = folderLabels[c.folder]
			g = {
				folder: c.folder,
				label: key ? formatMessage(messages[key]) : c.folder,
				attention: [],
				unchanged: [],
				excluded: [],
			}
			byFolder.set(c.folder, g)
		}
		const kind = kindOf(c, first)
		if (kind === 'new' || kind === 'changed') g.attention.push(c)
		else if (kind === 'unchanged') g.unchanged.push(c)
		else g.excluded.push(c)
	}
	return [...byFolder.values()].sort((a, b) =>
		a.folder === 'mods' ? -1 : b.folder === 'mods' ? 1 : a.folder.localeCompare(b.folder),
	)
})

function coreLabel(core: { game_version: string; loader: string; loader_version: string | null }) {
	const loader = core.loader.charAt(0).toUpperCase() + core.loader.slice(1)
	return core.loader === 'vanilla'
		? `Minecraft ${core.game_version}`
		: `Minecraft ${core.game_version} · ${loader} ${core.loader_version ?? ''}`.trim()
}

const nameChanged = computed(() => {
	const p = preview.value
	return !!p?.release_name && p.name !== p.release_name
})
const brandChanged = computed(() => nameChanged.value || !!preview.value?.icon_changed)

// Назва й іконка редагуються в налаштуваннях примірника (вкладка «Загальне»)
function changeBrand() {
	modal.value?.hide()
	void router.push({
		path: `/instance/${encodeURIComponent(instanceId.value)}`,
		query: { settings: 'general' },
	})
}

const coreChanged = computed(() => {
	const p = preview.value
	if (!p?.release_core) return false
	return (
		p.core.game_version !== p.release_core.game_version ||
		p.core.loader !== p.release_core.loader ||
		p.core.loader_version !== p.release_core.loader_version
	)
})

// Ядро змінюється в налаштуваннях примірника (вкладка «Установка») — ведемо туди
function changeCore() {
	modal.value?.hide()
	void router.push({
		path: `/instance/${encodeURIComponent(instanceId.value)}`,
		query: { settings: 'installation' },
	})
}

const hasAnyChange = computed(
	() =>
		coreChanged.value ||
		brandChanged.value ||
		folders.value.some((f) => f.attention.length > 0) ||
		(preview.value?.removed_from_instance.length ?? 0) > 0,
)

const includedCount = computed(() => Object.values(selected.value).filter(Boolean).length)
const includedMods = computed(
	() =>
		preview.value?.candidates.filter((c) => c.folder === 'mods' && selected.value[c.path]).length ??
		0,
)

function setGroup(items: PublishCandidate[], value: boolean) {
	for (const c of items) selected.value[c.path] = value
}

function formatSize(size: number | null) {
	if (!size) return ''
	return size >= 1e6 ? `${(size / 1e6).toFixed(1)} МБ` : `${Math.max(1, Math.round(size / 1e3))} КБ`
}

function relPath(c: PublishCandidate) {
	// У папці mods достатньо імені; у конфігів важливий підшлях
	return c.folder === 'mods' ? c.file_name : c.path.slice(c.folder.length + 1)
}

async function publish() {
	if (busy.value || !tag.value.trim() || !preview.value) return
	busy.value = true
	try {
		const excluded = preview.value.candidates
			.filter((c) => !selected.value[c.path])
			.map((c) => c.path)
		const release = await terrarium_publish_release({
			pack: pack.value,
			instance_id: instanceId.value,
			tag: tag.value.trim(),
			name: name.value.trim() || tag.value.trim(),
			body: notes.value.trim() || null,
			prerelease: toTest.value,
			excluded,
		})
		addNotification({
			type: 'success',
			title: formatMessage(release.prerelease ? messages.successTest : messages.success, {
				tag: release.tag,
			}),
			text: release.html_url,
		})
		emit('published', release)
		modal.value?.hide()
	} catch (err) {
		handleError(err)
	} finally {
		busy.value = false
	}
}

defineExpose({ show })
</script>

<template>
	<NewModal
		ref="modal"
		:header="pack === 'server' ? formatMessage(messages.headerServer) : formatMessage(messages.header)"
		scrollable
		width="48rem"
		max-width="calc(100vw - 2rem)"
	>
		<div class="flex flex-col gap-4">
			<p class="m-0 text-secondary">{{ formatMessage(messages.intro) }}</p>
			<div class="grid grid-cols-2 gap-4">
				<div class="flex flex-col gap-2">
					<label class="font-semibold text-contrast" for="terrarium-publish-tag">
						{{ formatMessage(messages.tagLabel) }}
					</label>
					<Input
						id="terrarium-publish-tag"
						v-model="tag"
						type="text"
						placeholder="v1.0.1"
						:spellcheck="false"
						wrapper-class="w-full"
					/>
				</div>
				<div class="flex flex-col gap-2">
					<label class="font-semibold text-contrast" for="terrarium-publish-name">
						{{ formatMessage(messages.nameLabel) }}
					</label>
					<Input id="terrarium-publish-name" v-model="name" type="text" wrapper-class="w-full" />
				</div>
			</div>
			<div class="flex flex-col gap-2">
				<span class="font-semibold text-contrast">{{ formatMessage(messages.targetLabel) }}</span>
				<div class="publish-target" role="radiogroup">
					<button
						type="button"
						role="radio"
						:aria-checked="toTest"
						class="publish-target__option"
						:class="{ 'is-active': toTest }"
						@click="toTest = true"
					>
						<TestIcon />
						<span class="publish-target__text">
							<strong>{{ formatMessage(messages.targetTest) }}</strong>
							<span>{{ formatMessage(messages.targetTestHint) }}</span>
						</span>
					</button>
					<button
						type="button"
						role="radio"
						:aria-checked="!toTest"
						class="publish-target__option"
						:class="{ 'is-active': !toTest }"
						@click="toTest = false"
					>
						<GlobeIcon />
						<span class="publish-target__text">
							<strong>{{ formatMessage(messages.targetAll) }}</strong>
							<span>{{ formatMessage(messages.targetAllHint) }}</span>
						</span>
					</button>
				</div>
			</div>
			<div class="flex flex-col gap-2">
				<label class="font-semibold text-contrast" for="terrarium-publish-notes">
					{{ formatMessage(messages.notesLabel) }}
				</label>
				<Textarea
					id="terrarium-publish-notes"
					v-model="notes"
					:placeholder="formatMessage(messages.notesPlaceholder)"
					wrapper-class="w-full"
				/>
			</div>

			<div v-if="loading" class="flex items-center gap-2 text-secondary">
				<SpinnerIcon class="animate-spin" /> {{ formatMessage(messages.loading) }}
			</div>

			<template v-else-if="preview">
				<p v-if="!preview.release_tag" class="m-0 text-sm text-secondary">
					{{ formatMessage(messages.firstRelease) }}
				</p>
				<p v-else-if="!hasAnyChange" class="m-0 text-sm text-secondary">
					{{ formatMessage(messages.noChanges, { tag: preview.release_tag }) }}
				</p>
				<p v-if="preview.release_prerelease" class="m-0 text-sm text-orange">
					{{ formatMessage(messages.baseIsTest, { tag: preview.release_tag }) }}
				</p>

				<section class="publish-group publish-brand" :class="{ 'publish-group--attention': brandChanged }">
					<header class="publish-group__head">
						<div class="publish-brand__row">
							<Avatar :src="iconUrl" size="3rem" pad-transparent-corners class="publish-brand__icon" />
							<div>
								<h3 class="publish-group__title">
									{{ formatMessage(messages.brandTitle) }}
									<span v-if="nameChanged" class="publish-item__tag is-changed">
										{{ formatMessage(messages.kindChanged) }}
									</span>
									<span v-if="preview.icon_changed" class="publish-item__tag is-changed">
										{{ formatMessage(messages.brandIconChanged) }}
									</span>
								</h3>
								<p class="publish-core__value">{{ preview.name }}</p>
								<p v-if="nameChanged" class="publish-group__hint">
									{{ formatMessage(messages.brandWas, { tag: preview.release_tag, name: preview.release_name }) }}
								</p>
								<p v-if="brandChanged" class="publish-group__hint">
									{{ formatMessage(messages.brandHint) }}
								</p>
							</div>
						</div>
						<Button type="outlined" size="sm" :disabled="busy" @click="changeBrand">
							<SettingsIcon /> {{ formatMessage(messages.brandChange) }}
						</Button>
					</header>
				</section>

				<section class="publish-group publish-core" :class="{ 'publish-group--attention': coreChanged }">
					<header class="publish-group__head">
						<div>
							<h3 class="publish-group__title">
								{{ formatMessage(messages.coreTitle) }}
								<span v-if="coreChanged" class="publish-item__tag is-changed">
									{{ formatMessage(messages.kindChanged) }}
								</span>
							</h3>
							<p class="publish-core__value">{{ coreLabel(preview.core) }}</p>
							<p v-if="coreChanged && preview.release_core" class="publish-group__hint">
								{{
									formatMessage(messages.coreWas, {
										tag: preview.release_tag,
										core: coreLabel(preview.release_core),
									})
								}}
							</p>
							<p v-if="coreChanged" class="publish-group__hint">
								{{ formatMessage(messages.coreHint) }}
							</p>
						</div>
						<Button type="outlined" size="sm" :disabled="busy" @click="changeCore">
							<SettingsIcon /> {{ formatMessage(messages.coreChange) }}
						</Button>
					</header>
				</section>

				<section
					v-for="g in folders"
					:key="g.folder"
					class="publish-group"
					:class="{ 'publish-group--attention': g.attention.length }"
				>
					<header class="publish-group__head">
						<h3 class="publish-group__title">
							{{ g.label }}
							<span v-if="g.attention.length" class="publish-group__count">{{ g.attention.length }}</span>
						</h3>
						<div v-if="g.attention.length > 1" class="publish-group__bulk">
							<button type="button" @click="setGroup(g.attention, true)">
								{{ formatMessage(messages.selectAll) }}
							</button>
							·
							<button type="button" @click="setGroup(g.attention, false)">
								{{ formatMessage(messages.selectNone) }}
							</button>
						</div>
					</header>

					<div v-if="g.attention.length" class="publish-group__list">
						<div
							v-for="c in g.attention"
							:key="c.path"
							class="publish-item"
							@click="selected[c.path] = !selected[c.path]"
						>
							<Checkbox
								:model-value="!!selected[c.path]"
								@click.stop
								@update:model-value="selected[c.path] = $event"
							/>
							<Avatar v-if="infoFor(c)" :src="infoFor(c)?.icon" size="2rem" class="publish-item__icon" />
							<FileIcon v-else class="publish-item__file" />
							<span class="publish-item__text">
								<span class="publish-item__name" :title="c.path">{{ infoFor(c)?.title ?? relPath(c) }}</span>
								<span v-if="infoFor(c)" class="publish-item__sub">{{ subtitleFor(c) }}</span>
							</span>
							<span class="publish-item__tag" :class="c.in_release && preview.release_tag ? 'is-changed' : 'is-new'">
								{{
									c.in_release && preview.release_tag
										? formatMessage(messages.kindChanged)
										: formatMessage(messages.kindNew)
								}}
							</span>
							<span class="publish-item__meta">{{ formatSize(c.size) }}</span>
						</div>
					</div>

					<div v-if="g.unchanged.length" class="publish-group__unchanged">
						<span>{{ formatMessage(messages.unchangedLine, { count: g.unchanged.length }) }}</span>
						<button type="button" @click="expandedUnchanged[g.folder] = !expandedUnchanged[g.folder]">
							{{
								expandedUnchanged[g.folder]
									? formatMessage(messages.hideUnchanged)
									: formatMessage(messages.showUnchanged)
							}}
							<ChevronDownIcon :class="{ 'rotate-180': expandedUnchanged[g.folder] }" />
						</button>
					</div>
					<div v-if="g.unchanged.length && expandedUnchanged[g.folder]" class="publish-group__list">
						<div
							v-for="c in g.unchanged"
							:key="c.path"
							class="publish-item"
							@click="selected[c.path] = !selected[c.path]"
						>
							<Checkbox
								:model-value="!!selected[c.path]"
								@click.stop
								@update:model-value="selected[c.path] = $event"
							/>
							<Avatar v-if="infoFor(c)" :src="infoFor(c)?.icon" size="2rem" class="publish-item__icon" />
							<FileIcon v-else class="publish-item__file" />
							<span class="publish-item__text">
								<span class="publish-item__name" :title="c.path">{{ infoFor(c)?.title ?? relPath(c) }}</span>
								<span v-if="infoFor(c)" class="publish-item__sub">{{ subtitleFor(c) }}</span>
							</span>
							<span class="publish-item__meta">{{ formatSize(c.size) }}</span>
						</div>
					</div>

					<div v-if="g.excluded.length" class="publish-group__list publish-group__list--muted">
						<div
							v-for="c in g.excluded"
							:key="c.path"
							class="publish-item"
							@click="selected[c.path] = !selected[c.path]"
						>
							<Checkbox
								:model-value="!!selected[c.path]"
								@click.stop
								@update:model-value="selected[c.path] = $event"
							/>
							<Avatar v-if="infoFor(c)" :src="infoFor(c)?.icon" size="2rem" class="publish-item__icon" />
							<FileIcon v-else class="publish-item__file" />
							<span class="publish-item__text">
								<span class="publish-item__name" :title="c.path">{{ infoFor(c)?.title ?? relPath(c) }}</span>
								<span v-if="infoFor(c)" class="publish-item__sub">{{ subtitleFor(c) }}</span>
							</span>
							<span class="publish-item__tag is-excluded">
								{{
									c.disabled
										? formatMessage(messages.kindDisabled)
										: formatMessage(messages.kindExcluded)
								}}
							</span>
							<span class="publish-item__meta">{{ formatSize(c.size) }}</span>
						</div>
					</div>
				</section>

				<section v-if="preview.removed_from_instance.length" class="publish-group publish-group--removed">
					<header class="publish-group__head">
						<h3 class="publish-group__title">{{ formatMessage(messages.removedTitle) }}</h3>
					</header>
					<ul class="publish-group__plain">
						<li v-for="path in preview.removed_from_instance" :key="path">{{ path }}</li>
					</ul>
				</section>

				<p class="m-0 font-semibold text-contrast">
					{{
						formatMessage(messages.summary, {
							included: includedCount,
							total: preview.candidates.length,
							mods: includedMods,
						})
					}}
				</p>
			</template>
		</div>
		<template #actions>
			<div class="flex items-center justify-end gap-2">
				<Button type="outlined" :disabled="busy" @click="modal?.hide()">
					<XIcon />
					{{ formatMessage(commonMessages.cancelButton) }}
				</Button>
				<Button
					type="colored"
					color="brand"
					:disabled="busy || loading || !preview || !tag.trim()"
					@click="publish"
				>
					<SpinnerIcon v-if="busy" class="animate-spin" />
					<RocketIcon v-else />
					{{ busy ? formatMessage(messages.publishing) : formatMessage(messages.publish) }}
				</Button>
			</div>
		</template>
	</NewModal>
</template>

<style scoped lang="scss">
.publish-target {
	display: grid;
	grid-template-columns: 1fr 1fr;
	gap: 0.5rem;
}

.publish-target__option {
	display: flex;
	align-items: center;
	gap: 0.6rem;
	padding: 0.6rem 0.75rem;
	border: 1px solid var(--color-button-border);
	border-radius: var(--radius-md);
	background: var(--color-button-bg);
	color: var(--color-base);
	font: inherit;
	text-align: left;
	cursor: pointer;
	transition:
		border-color 0.12s ease,
		background-color 0.12s ease;

	svg {
		width: 1.25rem;
		height: 1.25rem;
		flex-shrink: 0;
	}

	&.is-active {
		border-color: var(--color-brand);
		background: var(--color-brand-highlight);
		color: var(--color-contrast);
	}
}

.publish-target__text {
	display: flex;
	flex-direction: column;
	gap: 0.1rem;
	font-size: 0.85rem;

	span {
		color: var(--color-secondary);
		font-size: 0.78rem;
	}
}

.publish-group {
	border: 1px solid var(--color-divider);
	border-radius: var(--radius-lg);
	padding: 0.65rem 1rem;

	&--attention {
		border-color: color-mix(in srgb, var(--color-brand) 50%, transparent);
	}

	&--removed {
		border-color: color-mix(in srgb, var(--color-orange) 55%, transparent);
	}
}

.publish-brand__row {
	display: flex;
	align-items: center;
	gap: 0.85rem;
}

.publish-brand__icon {
	flex-shrink: 0;
	border-radius: var(--radius-md);
}

.publish-core__value {
	margin: 0.15rem 0 0;
	font-size: 0.95rem;
	font-weight: 600;
	color: var(--color-contrast);
}

.publish-group__head {
	display: flex;
	align-items: center;
	justify-content: space-between;
	gap: 1rem;
	margin-bottom: 0.35rem;
}

.publish-group__title {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	margin: 0;
	font-size: 1rem;
	font-weight: 700;
	color: var(--color-contrast);
}

.publish-group__count {
	padding: 0.05rem 0.5rem;
	border-radius: 9999px;
	background: var(--color-brand-highlight);
	color: var(--color-brand);
	font-size: 0.75rem;
}

.publish-group__bulk {
	display: flex;
	gap: 0.35rem;
	flex-shrink: 0;
	font-size: 0.85rem;
	color: var(--color-secondary);

	button {
		border: 0;
		padding: 0;
		background: none;
		color: var(--color-brand);
		font: inherit;
		cursor: pointer;

		&:hover {
			text-decoration: underline;
		}
	}
}

.publish-group__list {
	max-height: 18rem;
	overflow-y: auto;
	padding-right: 0.25rem;

	&--muted {
		opacity: 0.7;
	}
}

.publish-group__unchanged {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	font-size: 0.85rem;
	color: var(--color-secondary);

	button {
		display: inline-flex;
		align-items: center;
		gap: 0.15rem;
		border: 0;
		padding: 0;
		background: none;
		color: var(--color-brand);
		font: inherit;
		cursor: pointer;

		svg {
			width: 0.9rem;
			height: 0.9rem;
			transition: transform 0.15s ease;
		}
	}
}

.publish-group__plain {
	margin: 0;
	padding-left: 1.2rem;
	color: var(--color-base);
	font-size: 0.9rem;
}

.publish-item {
	display: flex;
	align-items: center;
	gap: 0.6rem;
	padding: 0.3rem 0.35rem;
	border-radius: var(--radius-md);
	cursor: pointer;
	user-select: none;

	&:hover {
		background: color-mix(in srgb, var(--color-contrast) 6%, transparent);
	}
}

.publish-item__icon {
	flex-shrink: 0;
	border-radius: var(--radius-sm);
}

.publish-item__file {
	flex-shrink: 0;
	width: 1.25rem;
	height: 1.25rem;
	margin: 0 0.375rem;
	color: var(--color-secondary);
}

.publish-item__text {
	flex: 1;
	min-width: 0;
	display: flex;
	flex-direction: column;
	line-height: 1.2;
}

.publish-item__name {
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
	font-size: 0.9rem;
	font-weight: 600;
	color: var(--color-contrast);
}

.publish-item__sub {
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
	font-size: 0.75rem;
	color: var(--color-secondary);
}

.publish-item__tag {
	flex-shrink: 0;
	padding: 0.05rem 0.45rem;
	border-radius: 9999px;
	font-size: 0.7rem;
	font-weight: 700;
	text-transform: uppercase;
	letter-spacing: 0.04em;

	&.is-new {
		background: var(--color-brand-highlight);
		color: var(--color-brand);
	}

	&.is-changed {
		background: var(--color-orange-bg);
		color: var(--color-orange);
	}

	&.is-excluded {
		background: color-mix(in srgb, var(--color-contrast) 10%, transparent);
		color: var(--color-secondary);
	}
}

.publish-item__meta {
	flex-shrink: 0;
	min-width: 3.5rem;
	text-align: right;
	font-size: 0.8rem;
	color: var(--color-secondary);
}
</style>
