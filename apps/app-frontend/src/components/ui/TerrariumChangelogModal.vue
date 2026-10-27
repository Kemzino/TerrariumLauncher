<template>
	<NewModal ref="modal" :header="formatMessage(messages.header)" max-width="720px">
		<div class="flex flex-col gap-3">
			<p class="m-0 text-sm text-secondary">{{ formatMessage(messages.hint) }}</p>
			<div v-if="loading" class="flex items-center gap-2 py-6 text-secondary">
				<SpinnerIcon class="animate-spin" />
				{{ formatMessage(messages.loading) }}
			</div>
			<p v-else-if="error" class="m-0 text-red">{{ error }}</p>
			<p v-else-if="entries.length === 0" class="m-0 text-secondary">
				{{ formatMessage(messages.empty) }}
			</p>
			<div v-else class="flex max-h-[65vh] flex-col gap-3 overflow-y-auto pr-1">
				<article
					v-for="entry in entries"
					:key="entry.tag"
					class="rounded-xl border border-solid border-surface-5 bg-button-bg p-4"
					:class="{ 'border-brand': entry.tag === installedTag }"
				>
					<header class="flex flex-wrap items-center gap-2">
						<h3 class="m-0 text-base font-semibold text-contrast">{{ entry.name }}</h3>
						<span
							class="rounded-full bg-surface-4 px-2 py-0.5 text-xs font-semibold text-secondary"
						>
							{{ entry.tag }}
						</span>
						<span
							v-if="entry.tag === installedTag"
							class="rounded-full bg-brand-highlight px-2 py-0.5 text-xs font-semibold text-brand"
						>
							{{ formatMessage(messages.installed) }}
						</span>
						<span
							v-else-if="isUnread(entry.tag)"
							class="rounded-full bg-highlight-orange px-2 py-0.5 text-xs font-semibold text-orange"
						>
							{{ formatMessage(messages.unread) }}
						</span>
						<span
							v-if="versionOf(entry.tag)?.latest"
							class="rounded-full bg-highlight-green px-2 py-0.5 text-xs font-semibold text-green"
						>
							{{ formatMessage(messages.current) }}
						</span>
						<span v-if="entry.published_at" class="ml-auto text-sm text-secondary">
							{{ formatDate(entry.published_at) }}
						</span>
					</header>
					<div
						v-if="entry.body.trim()"
						class="markdown-body mt-2 text-sm"
						v-html="renderString(entry.body)"
					/>
					<p v-else class="m-0 mt-2 text-sm text-secondary">
						{{ formatMessage(messages.noBody) }}
					</p>
					<!-- версія, яку можна поставити: собі, а адміну — ще й зробити актуальною для всіх -->
					<footer v-if="versionOf(entry.tag)" class="mt-3 flex flex-wrap gap-2">
						<Button :disabled="disabled || entry.tag === installedTag" @click="install(entry.tag)">
							<DownloadIcon /> {{ formatMessage(messages.install) }}
						</Button>
						<Button
							v-if="isAdmin && !versionOf(entry.tag)?.latest"
							:type="armedTag === entry.tag ? 'colored' : 'base'"
							:color="armedTag === entry.tag ? 'orange' : undefined"
							:disabled="settingLatest !== null"
							@click="makeLatest(entry.tag)"
						>
							<SpinnerIcon v-if="settingLatest === entry.tag" class="animate-spin" />
							<GlobeIcon v-else />
							{{
								armedTag === entry.tag
									? formatMessage(messages.makeLatestConfirm)
									: formatMessage(messages.makeLatest)
							}}
						</Button>
					</footer>
				</article>
			</div>
			<p v-if="isAdmin" class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.adminHint) }}
			</p>
		</div>
		<template #actions>
			<div class="flex justify-end gap-2">
				<Button v-if="repoUrl" type="outlined" @click="openUrl(repoUrl)">
					<ExternalIcon /> GitHub
				</Button>
				<Button type="colored" color="brand" @click="modal?.hide()">
					<CheckIcon /> {{ formatMessage(commonMessages.closeButton) }}
				</Button>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
// Terrarium: релізи збірки на головній — що нового в кожному, і сама версія: її можна поставити
// собі (напр. повернутись на попередню, якщо нова зламалась), а адмін може зробити старішу
// актуальною для всіх (GitHub make_latest, нічого не видаляється). Непрочитані позначаються,
// останній переглянутий тег зберігається локально.
import { CheckIcon, DownloadIcon, ExternalIcon, GlobeIcon, SpinnerIcon } from '@modrinth/assets'
import {
	Button,
	commonMessages,
	defineMessages,
	injectNotificationManager,
	NewModal,
	useVIntl,
} from '@modrinth/ui'
import { renderString } from '@modrinth/utils'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, ref } from 'vue'

import {
	type Channel,
	type PackKind,
	terrarium_list_release_versions,
	terrarium_list_releases,
	terrarium_set_latest_release,
	type TerrariumChangelogEntry,
	type TerrariumRelease,
	type TerrariumReleaseVersion,
} from '@/helpers/terrarium'

const { formatMessage } = useVIntl()
const { handleError, addNotification } = injectNotificationManager()

const messages = defineMessages({
	header: { id: 'terrarium.changelog.header', defaultMessage: 'Що нового у збірці' },
	hint: {
		id: 'terrarium.versions.hint',
		defaultMessage:
			'Можна поставити будь-яку опубліковану версію, напр. повернутись на попередню, якщо нова зламалась. Лаунчер і далі пропонуватиме оновитись до актуальної.',
	},
	loading: { id: 'terrarium.changelog.loading', defaultMessage: 'Завантажую список змін…' },
	empty: { id: 'terrarium.changelog.empty', defaultMessage: 'Релізів ще немає.' },
	installed: { id: 'terrarium.changelog.installed', defaultMessage: 'У тебе' },
	unread: { id: 'terrarium.changelog.unread', defaultMessage: 'Нове' },
	current: { id: 'terrarium.versions.current', defaultMessage: 'Актуальна' },
	noBody: {
		id: 'terrarium.changelog.no-body',
		defaultMessage: 'Автор не додав опис змін до цього релізу.',
	},
	install: { id: 'terrarium.versions.install', defaultMessage: 'Встановити' },
	makeLatest: { id: 'terrarium.versions.make-latest', defaultMessage: 'Зробити актуальною' },
	makeLatestConfirm: {
		id: 'terrarium.versions.make-latest-confirm',
		defaultMessage: 'Точно? Натисни ще раз',
	},
	madeLatest: {
		id: 'terrarium.versions.made-latest',
		defaultMessage: 'Актуальна версія тепер {tag} — гравцям запропонує на неї перейти',
	},
	adminHint: {
		id: 'terrarium.versions.admin-hint',
		defaultMessage:
			'«Зробити актуальною» — відкат для всіх: гравці побачать «Оновити» і перейдуть на цю версію. Новіші релізи лишаються на GitHub, їх можна повернути так само.',
	},
})

const props = defineProps<{
	pack: PackKind
	channel: Channel
	installedTag: string | null
	isAdmin: boolean
	/** Гра запущена або йде установка — ставити іншу версію зараз не можна */
	disabled: boolean
}>()

const emit = defineEmits<{
	seen: [tag: string]
	install: [release: TerrariumRelease]
	latestChanged: [release: TerrariumRelease]
}>()

const modal = ref<InstanceType<typeof NewModal>>()
const entries = ref<TerrariumChangelogEntry[]>([])
/** Версії, які можна поставити, за тегом (без чернеток і пре-релізів) */
const versions = ref(new Map<string, TerrariumReleaseVersion>())
const loading = ref(false)
const error = ref<string | null>(null)
const armedTag = ref<string | null>(null)
const settingLatest = ref<string | null>(null)
let armTimer: ReturnType<typeof setTimeout> | undefined
const repoUrl = computed(() =>
	entries.value[0]?.html_url.replace(/\/releases\/tag\/.*$/, '/releases'),
)

function versionOf(tag: string) {
	return versions.value.get(tag)
}

const seenKey = computed(() => `terrarium-changelog-seen:${props.pack}:${props.channel}`)
/** Тег, до якого гравець уже переглянув список змін */
const seenTag = ref<string | null>(readSeen())
function readSeen() {
	try {
		return localStorage.getItem(seenKey.value)
	} catch {
		return null
	}
}
function isUnread(tag: string) {
	return seenTag.value !== null && tag !== seenTag.value && isNewer(tag, seenTag.value)
}

/** Порівняння тегів v1.2.3 / 1.2.3 як версій; невідомі формати — за рядком */
function isNewer(a: string, b: string) {
	const pa = a
		.replace(/^v/, '')
		.split(/[.-]/)
		.map((x) => Number(x))
	const pb = b
		.replace(/^v/, '')
		.split(/[.-]/)
		.map((x) => Number(x))
	if (pa.some(Number.isNaN) || pb.some(Number.isNaN)) return a > b
	for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
		const da = pa[i] ?? 0
		const db = pb[i] ?? 0
		if (da !== db) return da > db
	}
	return false
}

function formatDate(value: string) {
	const date = new Date(value)
	return Number.isNaN(date.getTime()) ? value : date.toLocaleDateString()
}

async function loadVersions() {
	try {
		const list = await terrarium_list_release_versions(props.pack, props.channel, 30)
		versions.value = new Map(list.map((v) => [v.release.tag, v]))
	} catch {
		// без списку версій вікно все одно показує зміни, лише без кнопок
		versions.value = new Map()
	}
}

async function show() {
	armedTag.value = null
	modal.value?.show()
	seenTag.value = readSeen()
	loading.value = true
	error.value = null
	try {
		const [list] = await Promise.all([
			terrarium_list_releases(props.pack, props.channel, 30),
			loadVersions(),
		])
		entries.value = list
		// Переглянув — позначаємо найновіший як прочитаний
		const latest = entries.value[0]?.tag
		if (latest) {
			try {
				localStorage.setItem(seenKey.value, latest)
			} catch {
				/* без localStorage — просто не запам'ятаємо */
			}
			emit('seen', latest)
		}
	} catch (err) {
		error.value = String(err)
	} finally {
		loading.value = false
	}
}

function install(tag: string) {
	const version = versionOf(tag)
	if (!version) return
	modal.value?.hide()
	emit('install', version.release)
}

// Відкат для всіх — подвійне натискання, як і «Поширити для всіх»
async function makeLatest(tag: string) {
	const version = versionOf(tag)
	if (!props.isAdmin || !version || settingLatest.value !== null) return
	if (armedTag.value !== tag) {
		armedTag.value = tag
		clearTimeout(armTimer)
		armTimer = setTimeout(() => (armedTag.value = null), 6000)
		return
	}
	armedTag.value = null
	settingLatest.value = tag
	try {
		const latest = await terrarium_set_latest_release(props.pack, props.channel, version.release.id)
		addNotification({
			type: 'success',
			title: formatMessage(messages.madeLatest, { tag: latest.tag }),
		})
		emit('latestChanged', latest)
		await loadVersions()
	} catch (err) {
		handleError(err as Error)
	} finally {
		settingLatest.value = null
	}
}

defineExpose({ show })
</script>
