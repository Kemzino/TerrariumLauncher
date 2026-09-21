<template>
	<NewModal ref="modal" :header="formatMessage(messages.header)" max-width="720px">
		<div class="flex flex-col gap-3">
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
				</article>
			</div>
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
// Terrarium: список змін збірки — релізи GitHub на головній; непрочитані
// позначаються, останній переглянутий тег зберігається локально
import { CheckIcon, ExternalIcon, SpinnerIcon } from '@modrinth/assets'
import { Button, commonMessages, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { renderString } from '@modrinth/utils'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, ref } from 'vue'

import {
	type Channel,
	type PackKind,
	terrarium_list_releases,
	type TerrariumChangelogEntry,
} from '@/helpers/terrarium'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	header: { id: 'terrarium.changelog.header', defaultMessage: 'Що нового у збірці' },
	loading: { id: 'terrarium.changelog.loading', defaultMessage: 'Завантажую список змін…' },
	empty: { id: 'terrarium.changelog.empty', defaultMessage: 'Релізів ще немає.' },
	installed: { id: 'terrarium.changelog.installed', defaultMessage: 'У тебе' },
	unread: { id: 'terrarium.changelog.unread', defaultMessage: 'Нове' },
	noBody: {
		id: 'terrarium.changelog.no-body',
		defaultMessage: 'Автор не додав опис змін до цього релізу.',
	},
})

const props = defineProps<{
	pack: PackKind
	channel: Channel
	installedTag: string | null
}>()

const modal = ref<InstanceType<typeof NewModal>>()
const entries = ref<TerrariumChangelogEntry[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
const repoUrl = computed(() =>
	entries.value[0]?.html_url.replace(/\/releases\/tag\/.*$/, '/releases'),
)

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

async function show() {
	modal.value?.show()
	seenTag.value = readSeen()
	loading.value = true
	error.value = null
	try {
		entries.value = await terrarium_list_releases(props.pack, props.channel, 20)
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

const emit = defineEmits<{ seen: [tag: string] }>()

defineExpose({ show })
</script>
