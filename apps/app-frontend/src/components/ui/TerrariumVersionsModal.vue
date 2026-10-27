<template>
	<NewModal ref="modal" :header="formatMessage(messages.header)" max-width="720px">
		<div class="flex flex-col gap-3">
			<p class="m-0 text-sm text-secondary">{{ formatMessage(messages.hint) }}</p>
			<div v-if="loading" class="flex items-center gap-2 py-6 text-secondary">
				<SpinnerIcon class="animate-spin" />
				{{ formatMessage(messages.loading) }}
			</div>
			<p v-else-if="error" class="m-0 text-red">{{ error }}</p>
			<p v-else-if="versions.length === 0" class="m-0 text-secondary">
				{{ formatMessage(messages.empty) }}
			</p>
			<div v-else class="flex max-h-[60vh] flex-col gap-2 overflow-y-auto pr-1">
				<article
					v-for="version in versions"
					:key="version.release.id"
					class="flex flex-wrap items-center gap-2 rounded-xl border border-solid border-surface-5 bg-button-bg p-3"
					:class="{ 'border-brand': version.release.tag === installedTag }"
				>
					<div class="flex min-w-0 grow flex-col gap-1">
						<div class="flex flex-wrap items-center gap-2">
							<span class="font-semibold text-contrast">{{ version.release.name }}</span>
							<span
								class="rounded-full bg-surface-4 px-2 py-0.5 text-xs font-semibold text-secondary"
							>
								{{ version.release.tag }}
							</span>
							<span
								v-if="version.release.tag === installedTag"
								class="rounded-full bg-brand-highlight px-2 py-0.5 text-xs font-semibold text-brand"
							>
								{{ formatMessage(messages.installed) }}
							</span>
							<span
								v-if="version.latest"
								class="rounded-full bg-highlight-green px-2 py-0.5 text-xs font-semibold text-green"
							>
								{{ formatMessage(messages.current) }}
							</span>
						</div>
						<span v-if="version.release.published_at" class="text-sm text-secondary">
							{{ formatDate(version.release.published_at) }}
						</span>
					</div>
					<div class="flex flex-wrap gap-2">
						<Button
							:disabled="disabled || version.release.tag === installedTag"
							@click="install(version)"
						>
							<DownloadIcon /> {{ formatMessage(messages.install) }}
						</Button>
						<Button
							v-if="isAdmin && !version.latest"
							:type="armedId === version.release.id ? 'colored' : 'base'"
							:color="armedId === version.release.id ? 'orange' : undefined"
							:disabled="settingLatest !== null"
							@click="makeLatest(version)"
						>
							<SpinnerIcon v-if="settingLatest === version.release.id" class="animate-spin" />
							<GlobeIcon v-else />
							{{
								armedId === version.release.id
									? formatMessage(messages.makeLatestConfirm)
									: formatMessage(messages.makeLatest)
							}}
						</Button>
					</div>
				</article>
			</div>
			<p v-if="isAdmin" class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.adminHint) }}
			</p>
		</div>
		<template #actions>
			<div class="flex justify-end gap-2">
				<Button type="colored" color="brand" @click="modal?.hide()">
					<CheckIcon /> {{ formatMessage(commonMessages.closeButton) }}
				</Button>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
// Terrarium: відкат збірки. Гравець ставить будь-яку опубліковану версію у свій примірник;
// адмін може зробити старішу версію актуальною для всіх (GitHub make_latest, нічого не видаляється).
import { CheckIcon, DownloadIcon, GlobeIcon, SpinnerIcon } from '@modrinth/assets'
import {
	Button,
	commonMessages,
	defineMessages,
	injectNotificationManager,
	NewModal,
	useVIntl,
} from '@modrinth/ui'
import { ref } from 'vue'

import {
	type Channel,
	type PackKind,
	terrarium_list_release_versions,
	terrarium_set_latest_release,
	type TerrariumRelease,
	type TerrariumReleaseVersion,
} from '@/helpers/terrarium'

const { formatMessage } = useVIntl()
const { handleError, addNotification } = injectNotificationManager()

const messages = defineMessages({
	header: { id: 'terrarium.versions.header', defaultMessage: 'Версії збірки' },
	hint: {
		id: 'terrarium.versions.hint',
		defaultMessage:
			'Можна поставити будь-яку опубліковану версію, напр. повернутись на попередню, якщо нова зламалась. Лаунчер і далі пропонуватиме оновитись до актуальної.',
	},
	loading: { id: 'terrarium.versions.loading', defaultMessage: 'Завантажую версії…' },
	empty: { id: 'terrarium.versions.empty', defaultMessage: 'Опублікованих версій ще немає.' },
	installed: { id: 'terrarium.versions.installed', defaultMessage: 'У тебе' },
	current: { id: 'terrarium.versions.current', defaultMessage: 'Актуальна' },
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
	install: [release: TerrariumRelease]
	latestChanged: [release: TerrariumRelease]
}>()

const modal = ref<InstanceType<typeof NewModal>>()
const versions = ref<TerrariumReleaseVersion[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
const armedId = ref<number | null>(null)
const settingLatest = ref<number | null>(null)
let armTimer: ReturnType<typeof setTimeout> | undefined

function formatDate(value: string) {
	const date = new Date(value)
	return Number.isNaN(date.getTime()) ? value : date.toLocaleDateString()
}

async function load() {
	loading.value = true
	error.value = null
	try {
		versions.value = await terrarium_list_release_versions(props.pack, props.channel, 30)
	} catch (err) {
		error.value = String(err)
	} finally {
		loading.value = false
	}
}

async function show() {
	armedId.value = null
	modal.value?.show()
	await load()
}

function install(version: TerrariumReleaseVersion) {
	modal.value?.hide()
	emit('install', version.release)
}

// Відкат для всіх — подвійне натискання, як і «Поширити для всіх»
async function makeLatest(version: TerrariumReleaseVersion) {
	if (!props.isAdmin || settingLatest.value !== null) return
	const id = version.release.id
	if (armedId.value !== id) {
		armedId.value = id
		clearTimeout(armTimer)
		armTimer = setTimeout(() => (armedId.value = null), 6000)
		return
	}
	armedId.value = null
	settingLatest.value = id
	try {
		const latest = await terrarium_set_latest_release(props.pack, props.channel, id)
		addNotification({
			type: 'success',
			title: formatMessage(messages.madeLatest, { tag: latest.tag }),
		})
		emit('latestChanged', latest)
		await load()
	} catch (err) {
		handleError(err as Error)
	} finally {
		settingLatest.value = null
	}
}

defineExpose({ show })
</script>
