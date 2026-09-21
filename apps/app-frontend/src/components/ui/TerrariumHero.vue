<script setup lang="ts">
import {
	ArrowLeftRightIcon,
	BookOpenIcon,
	ChevronDownIcon,
	DiscordIcon,
	DownloadIcon,
	GithubIcon,
	GlobeIcon,
	HistoryIcon,
	ModrinthIcon,
	PackageOpenIcon,
	PlayIcon,
	RefreshCwIcon,
	RocketIcon,
	ServerStackIcon,
	SpinnerIcon,
	StopCircleIcon,
	TestIcon,
	UpdatedIcon,
} from '@modrinth/assets'
import { Button, defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { useQuery, useQueryClient } from '@tanstack/vue-query'
import { computed, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'

import terrariumLogo from '@/assets/terrarium/logo.png'
import AccountsCard from '@/components/ui/AccountsCard.vue'
import TerrariumBackdrop from '@/components/ui/TerrariumBackdrop.vue'
import TerrariumChangelogModal from '@/components/ui/TerrariumChangelogModal.vue'
import TerrariumInstancePicker from '@/components/ui/TerrariumInstancePicker.vue'
import TerrariumNewsWidget from '@/components/ui/TerrariumNewsWidget.vue'
import TerrariumPublishModal from '@/components/ui/TerrariumPublishModal.vue'
import TerrariumServerStatusWidget from '@/components/ui/TerrariumServerStatusWidget.vue'
import TerrariumSyncModal from '@/components/ui/TerrariumSyncModal.vue'
import { useAppEvent } from '@/composables/use-app-event'
import { useAppSettings } from '@/composables/use-app-settings'
import { handleSevereError } from '@/composables/use-error.js'
import { useTerrariumState } from '@/composables/use-terrarium-state'
import type { InstallJobSnapshot } from '@/generated/app-events/InstallJobSnapshot'
import type { InstallPhaseId } from '@/generated/app-events/InstallPhaseId'
import {
	install_create_modpack_instance,
	install_pack_to_existing_instance,
	wait_for_install_job,
} from '@/helpers/install'
import { getInstanceIconUrl, kill, run } from '@/helpers/instance'
import { get_by_instance_id } from '@/helpers/process'
import {
	type Channel,
	type PackKind,
	packStateKey,
	type PublishedRelease,
	terrarium_apply_pack_branding,
	terrarium_download_release,
	terrarium_fetch_latest_release,
	terrarium_prepare_pack_update,
	terrarium_promote_release,
	type TerrariumRelease,
} from '@/helpers/terrarium'
import { openTerrariumLink } from '@/helpers/terrarium-links'
import { instanceKeys, instanceListQueryOptions } from '@/pages/instance/query-options'
import { injectAppEvents } from '@/providers/app-events'

const { formatMessage } = useVIntl()
const { handleError, addNotification } = injectNotificationManager()
const appEvents = injectAppEvents()
const queryClient = useQueryClient()
const router = useRouter()

const messages = defineMessages({
	linkRules: { id: 'terrarium.links.rules', defaultMessage: 'Правила сервера' },
	keyRejectedHint: {
		id: 'terrarium.hero.key-rejected',
		defaultMessage:
			'GitHub відхилив ключ доступу (відкликано або перевидано) — онови його в налаштуваннях.',
	},
	noAccessHint: {
		id: 'terrarium.hero.no-access',
		defaultMessage: 'Репозиторій цієї збірки недоступний з поточним ключем.',
	},
	channelStable: { id: 'terrarium.hero.channel-stable', defaultMessage: 'Реліз' },
	channelTest: { id: 'terrarium.hero.channel-test', defaultMessage: 'Тест' },
	testBadge: { id: 'terrarium.hero.test-badge', defaultMessage: 'Тестова версія' },
	testerBadge: { id: 'terrarium.hero.tester-badge', defaultMessage: 'Тестер' },
	noTestRelease: {
		id: 'terrarium.hero.no-test-release',
		defaultMessage: 'Тестових версій немає — найновіша вже поширена для всіх',
	},
	testHint: {
		id: 'terrarium.hero.test-hint',
		defaultMessage: 'Тестова версія ставиться окремим примірником — основна гра не змінюється.',
	},
	promote: { id: 'terrarium.hero.promote', defaultMessage: 'Поширити для всіх' },
	promoteConfirm: { id: 'terrarium.hero.promote-confirm', defaultMessage: 'Точно? Натисни ще раз' },
	promoting: { id: 'terrarium.hero.promoting', defaultMessage: 'Поширюю…' },
	promoted: {
		id: 'terrarium.hero.promoted',
		defaultMessage: 'Версію {tag} поширено — гравці отримають оновлення',
	},
	publishWhilePlaying: {
		id: 'terrarium.hero.publish-while-playing',
		defaultMessage: 'Поки гра запущена, моди лежать у корені mods/ — закрий гру перед публікацією',
	},
	linkDiscord: { id: 'terrarium.links.discord', defaultMessage: 'Наш Discord' },
	linkGithub: { id: 'terrarium.links.github', defaultMessage: 'Вихідний код на GitHub' },
	title: { id: 'terrarium.hero.title', defaultMessage: 'Terrarium' },
	eyebrow: { id: 'terrarium.hero.eyebrow', defaultMessage: 'Minecraft · збірка спільноти' },
	eyebrowServer: { id: 'terrarium.hero.eyebrow-server', defaultMessage: 'Серверна збірка' },
	subtitle: {
		id: 'terrarium.hero.subtitle',
		defaultMessage: 'Наш сервер. Наша збірка. Один клік — і ти в грі.',
	},
	subtitleServer: {
		id: 'terrarium.hero.subtitle-server',
		defaultMessage: 'Те, що крутиться на сервері. Редагуй моди й публікуй — сервер підтягне.',
	},
	install: { id: 'terrarium.hero.install', defaultMessage: 'Встановити' },
	installServer: {
		id: 'terrarium.hero.install-server',
		defaultMessage: 'Завантажити для редагування',
	},
	update: { id: 'terrarium.hero.update', defaultMessage: 'Оновити' },
	play: { id: 'terrarium.hero.play', defaultMessage: 'Грати' },
	playOld: { id: 'terrarium.hero.play-old', defaultMessage: 'Грати без оновлення' },
	stop: { id: 'terrarium.hero.stop', defaultMessage: 'Зупинити' },
	mods: { id: 'terrarium.hero.mods', defaultMessage: 'Мої моди' },
	modsServer: { id: 'terrarium.hero.mods-server', defaultMessage: 'Моди сервера' },
	publish: { id: 'terrarium.hero.publish', defaultMessage: 'Опублікувати оновлення' },
	sync: { id: 'terrarium.hero.sync', defaultMessage: 'Синхронізувати' },
	syncTooltip: {
		id: 'terrarium.hero.sync-tooltip',
		defaultMessage: 'Перенести групи модів і конфіги між клієнтською та серверною збірками',
	},
	syncNeedBoth: {
		id: 'terrarium.hero.sync-need-both',
		defaultMessage: 'Щоб синхронізувати, встанови і клієнтську, і серверну збірку',
	},
	adminBadge: { id: 'terrarium.hero.admin-badge', defaultMessage: 'Адмін' },
	packClient: { id: 'terrarium.hero.pack-client', defaultMessage: 'Клієнт' },
	packServer: { id: 'terrarium.hero.pack-server', defaultMessage: 'Сервер' },
	scrollHint: { id: 'terrarium.hero.scroll-hint', defaultMessage: 'Інші збірки — нижче' },
	linkPrompt: {
		id: 'terrarium.hero.link-prompt',
		defaultMessage: 'Збірка вже стоїть як примірник?',
	},
	linkPlaceholder: { id: 'terrarium.hero.link-placeholder', defaultMessage: 'Обрати примірник…' },
	linkButton: { id: 'terrarium.hero.link-button', defaultMessage: 'Прив’язати' },
	unlinked: { id: 'terrarium.hero.unlinked', defaultMessage: 'Примірник відв’язано' },
	playingAs: { id: 'terrarium.hero.playing-as', defaultMessage: 'Граєш як' },
	checking: { id: 'terrarium.hero.checking', defaultMessage: 'Перевіряю оновлення…' },
	downloading: { id: 'terrarium.hero.downloading', defaultMessage: 'Завантажую збірку…' },
	installing: { id: 'terrarium.hero.installing', defaultMessage: 'Встановлюю…' },
	launching: { id: 'terrarium.hero.launching', defaultMessage: 'Запускаю…' },
	running: { id: 'terrarium.hero.running', defaultMessage: 'Гра запущена' },
	installedVersion: {
		id: 'terrarium.hero.installed-version',
		defaultMessage: 'Встановлено: {tag}',
	},
	latestVersion: {
		id: 'terrarium.hero.latest-version',
		defaultMessage: 'Доступна версія: {tag}',
	},
	upToDate: { id: 'terrarium.hero.up-to-date', defaultMessage: 'У тебе остання версія' },
	changelog: { id: 'terrarium.hero.changelog', defaultMessage: 'Що нового' },
	changelogUnread: {
		id: 'terrarium.hero.changelog-unread',
		defaultMessage: 'Є нові зміни, яких ти ще не бачив',
	},
	unknownLatest: {
		id: 'terrarium.hero.unknown-latest',
		defaultMessage: 'Встановлено {tag} · не вдалося перевірити, чи є новіша',
	},
	notInstalled: {
		id: 'terrarium.hero.not-installed',
		defaultMessage: 'Збірка ще не встановлена',
	},
	offlineHint: {
		id: 'terrarium.hero.offline-hint',
		defaultMessage: 'Немає з’єднання з GitHub — перевірити оновлення не вдалося.',
	},
})

const {
	state: terrariumState,
	activePack,
	activeChannel,
	activePackState: packState,
	isAdmin,
	isTester,
	reload: reloadState,
	patch: patchState,
	patchPack,
} = useTerrariumState()

const isServer = computed(() => activePack.value === 'server')
const appSettings = useAppSettings()
const showNews = computed(() => appSettings.getFeatureFlag('terrarium_show_news'))
const showServerStatus = computed(() => appSettings.getFeatureFlag('terrarium_show_server_status'))
const isTest = computed(() => activeChannel.value === 'test')
// У тестовому каналі реліз може виявитись звичайним — тестових версій просто нема
const hasTestRelease = computed(() => isTest.value && release.value?.prerelease === true)
const promoteArmed = ref(false)
const promoting = ref(false)
const release = ref<TerrariumRelease | null>(null)
const releaseError = ref<string | null>(null)
const checking = ref(true)

type Busy = null | 'downloading' | 'installing' | 'launching'
const busy = ref<Busy>(null)
const playing = ref(false)

const instancesQuery = useQuery(instanceListQueryOptions())
const instance = computed(() =>
	packState.value.instance_id
		? (instancesQuery.data.value ?? []).find((i) => i.id === packState.value.instance_id)
		: undefined,
)
const installed = computed(() => !!instance.value)
/**
 * Яка версія збірки реально стоїть у примірнику. Джерело правди — сам примірник
 * (версія з .mrpack, яку записав інсталятор); тег у terrarium.json — запасний
 * варіант, бо він міг розійтися з дійсністю (прив'язка чужого примірника тощо).
 */
const installedTag = computed<string | null>(() => {
	const link = instance.value?.link
	if (link?.type === 'imported_modpack' && link.version_number) {
		return link.version_number.startsWith('v') ? link.version_number : `v${link.version_number}`
	}
	return packState.value.installed_tag
})
const publishModal = ref<InstanceType<typeof TerrariumPublishModal> | null>(null)
const changelogModal = ref<InstanceType<typeof TerrariumChangelogModal> | null>(null)
// Непрочитаний список змін: останній реліз новіший за той, що гравець уже переглянув
const changelogSeenTag = ref<string | null>(null)
function readChangelogSeen() {
	try {
		changelogSeenTag.value = localStorage.getItem(
			`terrarium-changelog-seen:${activePack.value}:${activeChannel.value}`,
		)
	} catch {
		changelogSeenTag.value = null
	}
}
const changelogUnread = computed(
	() => !!release.value && release.value.tag !== changelogSeenTag.value,
)
watch([activePack, activeChannel], readChangelogSeen, { immediate: true })
function openChangelog() {
	changelogModal.value?.show()
}
const syncModal = ref<InstanceType<typeof TerrariumSyncModal> | null>(null)
// Синхронізація працює з примірниками поточного каналу; якщо в каналі збірки
// немає — беремо стабільну
function packInstanceId(kind: PackKind): string | null {
	const s = terrariumState.value
	const inChannel = s[packStateKey(kind, activeChannel.value)].instance_id
	return inChannel ?? s[packStateKey(kind, 'stable')].instance_id
}
const syncClientId = computed(() => packInstanceId('client'))
const syncServerId = computed(() => packInstanceId('server'))
const canSync = computed(() => !!syncClientId.value && !!syncServerId.value)

function openSync() {
	if (!syncClientId.value || !syncServerId.value) return
	syncModal.value?.show(syncClientId.value, syncServerId.value)
}

async function onSynced() {
	await queryClient.invalidateQueries({ queryKey: instanceKeys.list() })
}
// Будь-хто може прив'язати вже наявний примірник як «збірку Terrarium», а не качати її знову
const linkableInstances = computed(() => instancesQuery.data.value ?? [])
const updateAvailable = computed(
	() => installed.value && release.value !== null && release.value.tag !== installedTag.value,
)

async function refreshRelease() {
	checking.value = true
	releaseError.value = null
	try {
		release.value = await terrarium_fetch_latest_release(activePack.value, activeChannel.value)
	} catch (err) {
		release.value = null
		releaseError.value = String(err)
	} finally {
		checking.value = false
	}
}

async function refresh() {
	await reloadState()
	// Гравець без ключа не має бачити серверну збірку чи тестовий канал,
	// навіть якщо вони лишились активними
	if (!isTester.value && (activePack.value !== 'client' || activeChannel.value !== 'stable')) {
		await patchState({ active_pack: 'client', active_channel: 'stable' })
	}
	await refreshRelease()
}

async function switchPack(kind: PackKind) {
	if (kind === activePack.value || busy.value) return
	await patchState({ active_pack: kind }).catch(handleError)
	await refreshRelease()
	await checkProcess()
}

async function switchChannel(channel: Channel) {
	if (channel === activeChannel.value || busy.value) return
	promoteArmed.value = false
	await patchState({ active_channel: channel }).catch(handleError)
	await refreshRelease()
	await checkProcess()
}

// «Поширити для всіх»: тестовий реліз стає звичайним — гравці бачать оновлення
async function promote() {
	if (!release.value?.prerelease || !isAdmin.value) return
	if (!promoteArmed.value) {
		promoteArmed.value = true
		setTimeout(() => (promoteArmed.value = false), 6000)
		return
	}
	promoteArmed.value = false
	promoting.value = true
	const pack = activePack.value
	const tag = release.value.tag
	try {
		await terrarium_promote_release(pack, release.value.id)
		addNotification({
			type: 'success',
			title: formatMessage(messages.promoted, { tag }),
		})
		await refreshRelease()
	} catch (err) {
		handleError(err)
	} finally {
		promoting.value = false
	}
}

async function checkProcess() {
	playing.value = false
	if (!instance.value) return
	const processes = await get_by_instance_id(instance.value.id).catch(handleError)
	playing.value = Array.isArray(processes) && processes.length > 0
}

useAppEvent('process', (e) => {
	if (e.instance_id !== instance.value?.id) return
	if (e.event === 'launched') playing.value = true
	if (e.event === 'finished') playing.value = false
})

async function downloadLatest() {
	if (!release.value) throw new Error('Реліз ще не завантажено')
	busy.value = 'downloading'
	return await terrarium_download_release(release.value)
}

// Прогрес поточної установки/оновлення — з подій інсталятора
const activeJob = ref<InstallJobSnapshot | null>(null)
let activeJobId: string | null = null

useAppEvent('install_job', (job: InstallJobSnapshot) => {
	if (job.job_id === activeJobId) activeJob.value = job
})

const phaseLabels: Record<InstallPhaseId, string> = {
	preparing_instance: 'Готую примірник',
	resolving_pack: 'Читаю збірку',
	downloading_pack_file: 'Завантажую пакет',
	reading_pack_manifest: 'Читаю маніфест',
	downloading_content: 'Завантажую моди',
	extracting_overrides: 'Розпаковую конфіги',
	resolving_minecraft: 'Готую Minecraft',
	resolving_loader: 'Готую NeoForge',
	preparing_java: 'Готую Java',
	downloading_minecraft: 'Завантажую Minecraft',
	running_loader_processors: 'Встановлюю NeoForge',
	finalizing: 'Завершую',
	rolling_back: 'Відкочую зміни',
}

const installProgress = computed(() => {
	const job = activeJob.value
	if (!job) return null
	const phase = phaseLabels[job.phase] ?? job.phase
	const p = job.progress
	if (!p) return { phase, text: '', fraction: null as number | null }
	const bytes = p.secondary
	const fraction =
		bytes && bytes.total > 0
			? bytes.current / bytes.total
			: p.total > 0
				? p.current / p.total
				: null
	const mb = bytes
		? ` · ${(bytes.current / 1e6).toFixed(0)} / ${(bytes.total / 1e6).toFixed(0)} МБ`
		: ''
	return { phase, text: `${p.current}/${p.total}${mb}`, fraction }
})

function trackJob(job: InstallJobSnapshot) {
	activeJobId = job.job_id
	activeJob.value = job
}

function untrackJob() {
	activeJobId = null
	activeJob.value = null
}

async function install() {
	const pack = activePack.value
	try {
		const path = await downloadLatest()
		busy.value = 'installing'
		const job = await install_create_modpack_instance({ type: 'fromFile', path })
		trackJob(job)
		const finished = await wait_for_install_job(appEvents, job.job_id)
		if (finished.instance_id) {
			await terrarium_apply_pack_branding(finished.instance_id).catch(handleError)
		}
		await patchPack(
			pack,
			{
				instance_id: finished.instance_id,
				installed_tag: release.value!.tag,
			},
			activeChannel.value,
		)
		await queryClient.invalidateQueries({ queryKey: instanceKeys.list() })
	} catch (err) {
		handleError(err)
	} finally {
		untrackJob()
		busy.value = null
	}
}

async function update() {
	if (!instance.value) return
	const pack = activePack.value
	try {
		const path = await downloadLatest()
		busy.value = 'installing'
		// Моди з груп — тимчасово в корінь mods/, щоб оновлення бачило їх за
		// шляхами з попереднього пакета; apply_pack_branding повертає їх назад.
		await terrarium_prepare_pack_update(instance.value.id)
		let job
		try {
			job = await install_pack_to_existing_instance(instance.value.id, {
				type: 'fromFile',
				path,
			})
			trackJob(job)
			await wait_for_install_job(appEvents, job.job_id)
		} finally {
			await terrarium_apply_pack_branding(instance.value.id).catch(handleError)
		}
		await patchPack(pack, { installed_tag: release.value!.tag }, activeChannel.value)
		await queryClient.invalidateQueries({ queryKey: instanceKeys.list() })
	} catch (err) {
		handleError(err)
	} finally {
		untrackJob()
		busy.value = null
	}
}

async function play() {
	if (!instance.value) return
	busy.value = 'launching'
	try {
		await run(instance.value.id)
		playing.value = true
	} catch (err) {
		handleSevereError(err, { instanceId: instance.value.id })
	} finally {
		busy.value = null
	}
}

async function stop() {
	if (!instance.value) return
	playing.value = false
	await kill(instance.value.id).catch(handleError)
}

async function linkExisting(instanceId: string) {
	if (busy.value) return
	// Тег — лише з самого примірника (версія .mrpack); невідомо → null, і hero
	// запропонує оновлення замість «у тебе остання версія»
	const link = (instancesQuery.data.value ?? []).find((i) => i.id === instanceId)?.link
	const version = link?.type === 'imported_modpack' ? link.version_number : null
	await patchPack(
		activePack.value,
		{
			instance_id: instanceId,
			installed_tag: version ? (version.startsWith('v') ? version : `v${version}`) : null,
		},
		activeChannel.value,
	).catch(handleError)
	await checkProcess()
}

// Відв'язка нічого не видаляє — примірник лишається в бібліотеці, лаунчер просто
// перестає вважати його збіркою. Список виключених файлів зберігаємо.
async function unlinkInstance() {
	if (busy.value || playing.value) return
	await patchPack(
		activePack.value,
		{ instance_id: null, installed_tag: null },
		activeChannel.value,
	).catch(handleError)
	playing.value = false
	addNotification({ type: 'success', title: formatMessage(messages.unlinked) })
}

function openPublish() {
	if (!instance.value) return
	publishModal.value?.show(
		activePack.value,
		instance.value.id,
		installedTag.value,
		getInstanceIconUrl(instance.value.icon_path),
	)
}

function onPublished(published: PublishedRelease) {
	// Адмін щойно опублікував те, що в нього встановлено — стан уже оновив Rust;
	// перечитуємо його та реліз, щоб hero не показував «є оновлення».
	void reloadState().then(refreshRelease).catch(handleError)
	if (release.value) release.value = { ...release.value, tag: published.tag }
}

function openMods() {
	if (!instance.value) return
	router.push(`/instance/${encodeURIComponent(instance.value.id)}`)
}

const busyLabel = computed(() => {
	switch (busy.value) {
		case 'downloading':
			return formatMessage(messages.downloading)
		case 'installing':
			return installProgress.value?.phase ?? formatMessage(messages.installing)
		case 'launching':
			return formatMessage(messages.launching)
		default:
			return null
	}
})

// Якщо ключ додали/прибрали в налаштуваннях — перечитати реліз під актуальну збірку
watch(isTester, async (tester) => {
	if (tester) return
	// Ключ видалено: канал/збірку вже могли скинути в налаштуваннях, тому не
	// покладаємось на switch* (вони нічого не роблять, якщо значення те саме) —
	// перечитуємо реліз завжди, інакше hero показує версію з тест-репо
	if (activeChannel.value !== 'stable' || activePack.value !== 'client') {
		await patchState({ active_pack: 'client', active_channel: 'stable' }).catch(handleError)
	}
	await refreshRelease()
	await checkProcess()
})

onMounted(async () => {
	await refresh()
	await checkProcess()
	// Якщо лаунчер упав посеред оновлення збірки — доводимо групи модів до ладу
	if (instance.value) {
		await terrarium_apply_pack_branding(instance.value.id).catch(() => {})
	}
})
</script>

<template>
	<section class="terrarium-hero" :class="{ 'terrarium-hero--server': isServer }">
		<TerrariumBackdrop />

		<div v-if="isTester" class="terrarium-hero__switches">
			<div class="terrarium-hero__switch" role="tablist">
				<button
					type="button"
					role="tab"
					:aria-selected="!isServer"
					:class="{ active: !isServer }"
					:disabled="!!busy"
					@click="switchPack('client')"
				>
					<PlayIcon /> {{ formatMessage(messages.packClient) }}
				</button>
				<button
					type="button"
					role="tab"
					:aria-selected="isServer"
					:class="{ active: isServer }"
					:disabled="!!busy"
					@click="switchPack('server')"
				>
					<ServerStackIcon /> {{ formatMessage(messages.packServer) }}
				</button>
			</div>
			<div class="terrarium-hero__switch terrarium-hero__switch--channel" role="tablist">
				<button
					type="button"
					role="tab"
					:aria-selected="!isTest"
					:class="{ active: !isTest }"
					:disabled="!!busy"
					@click="switchChannel('stable')"
				>
					<GlobeIcon /> {{ formatMessage(messages.channelStable) }}
				</button>
				<button
					type="button"
					role="tab"
					:aria-selected="isTest"
					:class="{ active: isTest, 'is-test': isTest }"
					:disabled="!!busy"
					@click="switchChannel('test')"
				>
					<TestIcon /> {{ formatMessage(messages.channelTest) }}
				</button>
			</div>
		</div>

		<div class="terrarium-hero__content">
			<div class="terrarium-hero__main">
				<div class="terrarium-hero__badges">
					<span class="terrarium-hero__eyebrow">
						{{ isServer ? formatMessage(messages.eyebrowServer) : formatMessage(messages.eyebrow) }}
					</span>
					<span v-if="isAdmin" class="terrarium-hero__eyebrow terrarium-hero__eyebrow--admin">
						{{ formatMessage(messages.adminBadge) }}
					</span>
					<span v-else-if="isTester" class="terrarium-hero__eyebrow terrarium-hero__eyebrow--admin">
						{{ formatMessage(messages.testerBadge) }}
					</span>
					<span v-if="isTest" class="terrarium-hero__eyebrow terrarium-hero__eyebrow--test">
						<TestIcon /> {{ formatMessage(messages.testBadge) }}
					</span>
				</div>
				<h1 class="terrarium-hero__title">
					<img
						:src="terrariumLogo"
						:alt="formatMessage(messages.title)"
						class="terrarium-hero__logo"
					/>
				</h1>
				<p class="terrarium-hero__subtitle">
					{{ isServer ? formatMessage(messages.subtitleServer) : formatMessage(messages.subtitle) }}
				</p>

				<p class="terrarium-hero__status">
					<template v-if="checking">
						<SpinnerIcon class="animate-spin" /> {{ formatMessage(messages.checking) }}
					</template>
					<template v-else-if="busyLabel">
						<SpinnerIcon class="animate-spin" /> {{ busyLabel }}
						<span v-if="installProgress?.text" class="terrarium-hero__muted">
							· {{ installProgress.text }}
						</span>
					</template>
					<template v-else-if="playing">
						<PlayIcon class="text-brand" /> {{ formatMessage(messages.running) }}
					</template>
					<template v-else-if="!installed">
						<PackageOpenIcon /> {{ formatMessage(messages.notInstalled) }}
						<span v-if="release" class="terrarium-hero__muted">
							· {{ formatMessage(messages.latestVersion, { tag: release.tag }) }}
						</span>
					</template>
					<template v-else-if="updateAvailable">
						<UpdatedIcon class="text-brand" />
						{{ formatMessage(messages.latestVersion, { tag: release!.tag }) }}
						<span class="terrarium-hero__muted">
							· {{ formatMessage(messages.installedVersion, { tag: installedTag ?? '—' }) }}
						</span>
					</template>
					<template v-else-if="!release">
						<PackageOpenIcon class="text-orange" />
						{{ formatMessage(messages.unknownLatest, { tag: installedTag ?? '—' }) }}
					</template>
					<template v-else>
						<UpdatedIcon class="text-brand" /> {{ formatMessage(messages.upToDate) }}
						<span v-if="installedTag" class="terrarium-hero__muted"> · {{ installedTag }} </span>
					</template>
				</p>
				<div v-if="installProgress" class="terrarium-hero__bar" aria-hidden="true">
					<div
						class="terrarium-hero__bar-fill"
						:class="{ 'is-indeterminate': installProgress.fraction === null }"
						:style="
							installProgress.fraction !== null
								? { width: `${Math.round(installProgress.fraction * 100)}%` }
								: {}
						"
					></div>
				</div>
				<p v-if="releaseError && !checking" class="terrarium-hero__hint">
					{{
						releaseError.includes('401')
							? formatMessage(messages.keyRejectedHint)
							: releaseError.includes('404')
								? formatMessage(messages.noAccessHint)
								: formatMessage(messages.offlineHint)
					}}
				</p>
				<p
					v-else-if="isTest && !checking && release && !release.prerelease"
					class="terrarium-hero__hint"
				>
					{{ formatMessage(messages.noTestRelease) }}
				</p>
				<p v-else-if="isTest && !checking" class="terrarium-hero__hint">
					{{ formatMessage(messages.testHint) }}
				</p>

				<!-- Вибір примірника доступний усім: у гравця може бути кілька копій збірки
				     (стара версія, свій примірник) — він сам обирає, яка «головна». -->
				<div class="terrarium-hero__link">
					<TerrariumInstancePicker
						:instances="linkableInstances"
						:current="instance"
						:disabled="!!busy || playing"
						@select="linkExisting"
						@unlink="unlinkInstance"
					/>
				</div>

				<div class="terrarium-hero__actions">
					<template v-if="playing">
						<Button type="colored" color="red" size="xl" @click="stop">
							<StopCircleIcon /> {{ formatMessage(messages.stop) }}
						</Button>
					</template>
					<template v-else-if="!installed">
						<Button
							type="colored"
							color="brand"
							size="xl"
							class="terrarium-hero__cta"
							:disabled="checking || !!busy || !release"
							@click="install"
						>
							<DownloadIcon />
							{{
								isServer ? formatMessage(messages.installServer) : formatMessage(messages.install)
							}}
						</Button>
					</template>
					<template v-else-if="updateAvailable">
						<Button
							type="colored"
							color="brand"
							size="xl"
							class="terrarium-hero__cta"
							:disabled="!!busy"
							@click="update"
						>
							<RefreshCwIcon /> {{ formatMessage(messages.update) }}
						</Button>
						<Button size="xl" :disabled="!!busy" @click="play">
							<PlayIcon /> {{ formatMessage(messages.playOld) }}
						</Button>
					</template>
					<template v-else>
						<Button
							type="colored"
							color="brand"
							size="xl"
							class="terrarium-hero__cta"
							:disabled="checking || !!busy"
							@click="play"
						>
							<PlayIcon /> {{ formatMessage(messages.play) }}
						</Button>
					</template>

					<Button v-if="installed" size="xl" :disabled="!!busy" @click="openMods">
						<PackageOpenIcon />
						{{ isServer ? formatMessage(messages.modsServer) : formatMessage(messages.mods) }}
					</Button>
					<Button
						v-tooltip="changelogUnread ? formatMessage(messages.changelogUnread) : undefined"
						size="xl"
						class="relative"
						@click="openChangelog"
					>
						<HistoryIcon /> {{ formatMessage(messages.changelog) }}
						<span
							v-if="changelogUnread"
							class="absolute -right-1 -top-1 size-3 rounded-full bg-orange ring-2 ring-[var(--color-raised-bg)]"
							aria-hidden="true"
						></span>
					</Button>
					<!-- Публікація — лише для клієнтської збірки: серверна на сервер із GitHub
					     не підтягується, тож кнопка там лише вводила б в оману. -->
					<Button
						v-if="isAdmin && installed && !isServer"
						v-tooltip="playing ? formatMessage(messages.publishWhilePlaying) : undefined"
						size="xl"
						:type="isServer && !updateAvailable ? 'colored' : 'base'"
						:color="isServer && !updateAvailable ? 'brand' : undefined"
						:disabled="!!busy || playing"
						@click="openPublish"
					>
						<RocketIcon /> {{ formatMessage(messages.publish) }}
					</Button>
					<Button
						v-if="isAdmin"
						v-tooltip="
							canSync ? formatMessage(messages.syncTooltip) : formatMessage(messages.syncNeedBoth)
						"
						size="xl"
						:disabled="!!busy || playing || !canSync"
						@click="openSync"
					>
						<ArrowLeftRightIcon /> {{ formatMessage(messages.sync) }}
					</Button>
					<Button
						v-if="isAdmin && hasTestRelease"
						size="xl"
						:type="promoteArmed ? 'colored' : 'base'"
						:color="promoteArmed ? 'orange' : undefined"
						:disabled="!!busy || promoting"
						@click="promote"
					>
						<SpinnerIcon v-if="promoting" class="animate-spin" />
						<GlobeIcon v-else />
						{{
							promoting
								? formatMessage(messages.promoting)
								: promoteArmed
									? formatMessage(messages.promoteConfirm)
									: formatMessage(messages.promote)
						}}
					</Button>
				</div>

				<p class="terrarium-hero__powered">
					<span>Launcher powered by</span>
					<ModrinthIcon class="terrarium-hero__powered-icon" />
					<span>Modrinth</span>
				</p>
			</div>

			<div class="terrarium-hero__side">
				<div class="terrarium-hero__social">
					<button
						v-tooltip="formatMessage(messages.linkDiscord)"
						type="button"
						class="terrarium-hero__social-btn terrarium-hero__social-btn--discord"
						:aria-label="formatMessage(messages.linkDiscord)"
						@click="openTerrariumLink('discord')"
					>
						<DiscordIcon />
					</button>
					<button
						v-tooltip="formatMessage(messages.linkRules)"
						type="button"
						class="terrarium-hero__social-btn"
						:aria-label="formatMessage(messages.linkRules)"
						@click="openTerrariumLink('rules')"
					>
						<BookOpenIcon />
					</button>
					<button
						v-tooltip="formatMessage(messages.linkGithub)"
						type="button"
						class="terrarium-hero__social-btn"
						:aria-label="formatMessage(messages.linkGithub)"
						@click="openTerrariumLink('github')"
					>
						<GithubIcon />
					</button>
				</div>
				<div class="terrarium-hero__panels">
					<TerrariumServerStatusWidget v-if="showServerStatus" class="terrarium-hero__panel" />
					<TerrariumNewsWidget v-if="showNews" class="terrarium-hero__panel" />
					<aside class="terrarium-hero__account">
						<h3 class="terrarium-hero__account-title">{{ formatMessage(messages.playingAs) }}</h3>
						<Suspense>
							<AccountsCard />
						</Suspense>
					</aside>
				</div>
			</div>
		</div>

		<div class="terrarium-hero__scroll-hint" aria-hidden="true">
			<span>{{ formatMessage(messages.scrollHint) }}</span>
			<ChevronDownIcon />
		</div>

		<TerrariumPublishModal ref="publishModal" @published="onPublished" />
		<TerrariumChangelogModal
			ref="changelogModal"
			:pack="activePack"
			:channel="activeChannel"
			:installed-tag="installedTag"
			@seen="(tag) => (changelogSeenTag = tag)"
		/>
		<TerrariumSyncModal ref="syncModal" @synced="onSynced" />
	</section>
</template>

<style scoped lang="scss">
.terrarium-hero {
	position: relative;
	min-height: 100%;
	display: flex;
	align-items: flex-end;
	overflow: hidden;
}

.terrarium-hero__badges {
	display: flex;
	flex-wrap: wrap;
	align-items: center;
	gap: 0.5rem;
}

.terrarium-hero__switches {
	position: absolute;
	top: 1.25rem;
	left: 3.5rem;
	z-index: 2;
	display: flex;
	flex-wrap: wrap;
	gap: 0.5rem;
}

.terrarium-hero__switch {
	display: inline-flex;
	padding: 0.2rem;
	border-radius: 9999px;
	background: var(--terrarium-glass);
	border: 1px solid color-mix(in srgb, var(--color-contrast) 12%, transparent);
	backdrop-filter: blur(10px);

	button {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		padding: 0.3rem 0.8rem;
		border: 0;
		border-radius: 9999px;
		background: transparent;
		color: var(--color-secondary);
		font: inherit;
		font-size: 0.8rem;
		font-weight: 700;
		cursor: pointer;
		transition:
			background-color 0.12s ease,
			color 0.12s ease;

		svg {
			width: 0.95rem;
			height: 0.95rem;
		}

		&.active {
			background: var(--color-brand);
			color: var(--color-accent-contrast);
		}

		&.active.is-test {
			background: var(--color-orange);
			color: #fff;
		}

		&:disabled {
			cursor: default;
			opacity: 0.6;
		}
	}
}

.terrarium-hero__eyebrow--admin {
	background: var(--color-brand-highlight);
	border-color: var(--color-brand);
	color: var(--color-brand);
}

.terrarium-hero__eyebrow--test {
	background: color-mix(in srgb, var(--color-orange) 18%, transparent);
	border-color: var(--color-orange);
	color: var(--color-orange);
}

.terrarium-hero__scroll-hint {
	position: absolute;
	left: 50%;
	bottom: 0.75rem;
	transform: translateX(-50%);
	z-index: 1;
	display: flex;
	flex-direction: column;
	align-items: center;
	gap: 0.1rem;
	font-size: 0.72rem;
	font-weight: 600;
	letter-spacing: 0.06em;
	text-transform: uppercase;
	color: var(--color-secondary);
	opacity: 0.7;
	pointer-events: none;

	svg {
		width: 1.1rem;
		height: 1.1rem;
		animation: scroll-hint-bounce 1.8s ease-in-out infinite;
	}
}

@keyframes scroll-hint-bounce {
	0%,
	100% {
		transform: translateY(0);
	}
	50% {
		transform: translateY(4px);
	}
}

.terrarium-hero__content {
	position: relative;
	z-index: 1;
	width: 100%;
	display: grid;
	grid-template-columns: minmax(0, 1fr) auto;
	align-items: end;
	gap: 2rem;
	padding: 3rem 3.5rem 3rem;
}

.terrarium-hero__eyebrow {
	display: inline-flex;
	align-items: center;
	gap: 0.5rem;
	padding: 0.3rem 0.75rem;
	border-radius: 9999px;
	background: var(--terrarium-yellow-soft);
	border: 1px solid color-mix(in srgb, var(--terrarium-yellow) 45%, transparent);
	color: var(--terrarium-yellow);
	font-size: 0.8rem;
	font-weight: 700;
	letter-spacing: 0.08em;
	text-transform: uppercase;
}

.terrarium-hero__title {
	margin: 0.9rem 0 0;
	line-height: 0;
}

.terrarium-hero__logo {
	display: block;
	width: clamp(16rem, 28vw, 24rem);
	height: auto;
	filter: drop-shadow(0 10px 30px rgba(0, 0, 0, 0.45));
	user-select: none;
	-webkit-user-drag: none;
}

.terrarium-hero__subtitle {
	margin: 0.75rem 0 1.5rem;
	max-width: 34rem;
	font-size: 1.15rem;
	line-height: 1.5;
	color: var(--color-base);
	text-shadow: 0 1px 12px rgba(0, 0, 0, 0.35);
}

.terrarium-hero__status {
	display: flex;
	flex-wrap: wrap;
	align-items: center;
	gap: 0.45rem;
	margin: 0;
	font-weight: 600;
	color: var(--color-contrast);

	svg {
		width: 1.15rem;
		height: 1.15rem;
	}
}

.terrarium-hero__muted {
	color: var(--color-secondary);
	font-weight: 500;
}

.terrarium-hero__bar {
	width: min(28rem, 100%);
	height: 6px;
	margin-top: 0.6rem;
	border-radius: 9999px;
	background: color-mix(in srgb, var(--color-contrast) 15%, transparent);
	overflow: hidden;
}

.terrarium-hero__bar-fill {
	height: 100%;
	border-radius: inherit;
	background: var(--color-brand);
	transition: width 0.3s ease;

	&.is-indeterminate {
		width: 35%;
		animation: bar-indeterminate 1.2s ease-in-out infinite;
	}
}

@keyframes bar-indeterminate {
	0% {
		transform: translateX(-100%);
	}
	100% {
		transform: translateX(300%);
	}
}

.terrarium-hero__hint {
	margin: 0.4rem 0 0;
	font-size: 0.9rem;
	color: var(--color-orange);
}

.terrarium-hero__link {
	display: flex;
	flex-wrap: wrap;
	align-items: center;
	gap: 0.5rem;
	margin-top: 0.9rem;
	font-size: 0.9rem;
	color: var(--color-secondary);
}

.terrarium-hero__actions {
	display: flex;
	flex-wrap: wrap;
	gap: 0.75rem;
	align-items: center;
	margin-top: 1.5rem;
}

.terrarium-hero__cta {
	box-shadow: 0 8px 30px var(--color-brand-shadow);
}

.terrarium-hero__side {
	display: flex;
	align-items: flex-end;
	justify-content: flex-end;
	gap: 0.75rem;
}

.terrarium-hero__social {
	display: flex;
	flex-direction: column;
	gap: 0.5rem;
	padding-bottom: 0.25rem;
}

.terrarium-hero__social-btn {
	display: inline-flex;
	align-items: center;
	justify-content: center;
	width: 2.5rem;
	height: 2.5rem;
	border: 1px solid color-mix(in srgb, var(--color-contrast) 14%, transparent);
	border-radius: 999px;
	background: var(--terrarium-glass);
	color: var(--color-contrast);
	cursor: pointer;
	backdrop-filter: blur(10px);
	transition:
		background-color 0.15s ease,
		color 0.15s ease,
		transform 0.15s ease,
		border-color 0.15s ease;
}

.terrarium-hero__social-btn svg {
	width: 1.25rem;
	height: 1.25rem;
}

.terrarium-hero__social-btn:hover,
.terrarium-hero__social-btn:focus-visible {
	background: var(--color-brand);
	color: var(--color-accent-contrast);
	border-color: var(--color-brand);
	transform: translateY(-2px);
}

.terrarium-hero__social-btn--discord:hover,
.terrarium-hero__social-btn--discord:focus-visible {
	background: #5865f2;
	border-color: #5865f2;
	color: #fff;
}

.terrarium-hero__powered {
	display: flex;
	align-items: center;
	gap: 0.35rem;
	margin: 1.75rem 0 0;
	font-size: 0.78rem;
	font-weight: 500;
	color: var(--color-secondary);
	opacity: 0.85;
}

.terrarium-hero__powered-icon {
	width: 0.95rem;
	height: 0.95rem;
	color: var(--color-brand);
}

.terrarium-hero__panels {
	display: flex;
	flex-direction: column;
	gap: 0.75rem;
	width: 300px;
}

// Віджети в hero — у тому ж «скляному» стилі, що й картка акаунта
.terrarium-hero__panel {
	background: var(--terrarium-glass) !important;
	border-color: color-mix(in srgb, var(--color-contrast) 12%, transparent) !important;
	backdrop-filter: blur(14px);
	box-shadow: 0 12px 40px rgba(0, 0, 0, 0.25);
}

.terrarium-hero__account {
	width: 300px;
	padding: 1.25rem;
	border-radius: var(--radius-lg);
	background: var(--terrarium-glass);
	border: 1px solid color-mix(in srgb, var(--color-contrast) 12%, transparent);
	backdrop-filter: blur(14px);
	box-shadow: 0 12px 40px rgba(0, 0, 0, 0.25);
}

.terrarium-hero__account-title {
	margin: 0 0 0.25rem;
	font-size: 0.8rem;
	font-weight: 700;
	letter-spacing: 0.08em;
	text-transform: uppercase;
	color: var(--color-secondary);
}

@media (max-width: 960px) {
	.terrarium-hero__content {
		grid-template-columns: 1fr;
		padding: 2rem 1.5rem;
	}
}
</style>
