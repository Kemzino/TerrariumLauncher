<script setup lang="ts">
// Terrarium: опис останнього релізу збірки прямо на головній — гравець бачить, що змінилось,
// не відкриваючи вікно «Що нового». Усі релізи — кнопкою «Усі зміни».
import { HistoryIcon, SpinnerIcon } from '@modrinth/assets'
import { Button, defineMessages, useVIntl } from '@modrinth/ui'
import { renderString } from '@modrinth/utils'
import { ref, watch } from 'vue'

import {
	type Channel,
	type PackKind,
	terrarium_list_releases,
	type TerrariumChangelogEntry,
} from '@/helpers/terrarium'

const props = defineProps<{
	pack: PackKind
	channel: Channel
	installedTag: string | null
}>()
const emit = defineEmits<{ openAll: [] }>()

const { formatMessage } = useVIntl()
const messages = defineMessages({
	title: { id: 'terrarium.latest.title', defaultMessage: 'Що нового у {tag}' },
	installed: { id: 'terrarium.changelog.installed', defaultMessage: 'У тебе' },
	notInstalled: { id: 'terrarium.latest.not-installed', defaultMessage: 'Ще не встановлено' },
	all: { id: 'terrarium.latest.all', defaultMessage: 'Усі зміни' },
	loading: { id: 'terrarium.changelog.loading', defaultMessage: 'Завантажую список змін…' },
	empty: {
		id: 'terrarium.changelog.no-body',
		defaultMessage: 'Автор не додав опис змін до цього релізу.',
	},
})

const entry = ref<TerrariumChangelogEntry | null>(null)
const loading = ref(true)
const error = ref<string | null>(null)

async function load() {
	loading.value = true
	error.value = null
	try {
		entry.value = (await terrarium_list_releases(props.pack, props.channel, 1))[0] ?? null
	} catch (err) {
		error.value = String(err)
	} finally {
		loading.value = false
	}
}

watch(() => [props.pack, props.channel], load, { immediate: true })

function formatDate(value: string | null) {
	if (!value) return ''
	const date = new Date(value)
	return Number.isNaN(date.getTime()) ? value : date.toLocaleDateString()
}
</script>

<template>
	<section class="terrarium-widget terrarium-latest">
		<header class="terrarium-widget__head">
			<h3 class="terrarium-widget__title">
				<HistoryIcon />
				{{
					entry ? formatMessage(messages.title, { tag: entry.tag }) : formatMessage(messages.all)
				}}
			</h3>
			<span v-if="entry" class="terrarium-latest__meta">
				<span
					v-if="entry.tag === props.installedTag"
					class="rounded-full bg-brand-highlight px-2 py-0.5 text-xs font-semibold text-brand"
				>
					{{ formatMessage(messages.installed) }}
				</span>
				{{ formatDate(entry.published_at) }}
			</span>
		</header>
		<div class="terrarium-latest__body">
			<p v-if="loading" class="flex items-center gap-2 text-secondary">
				<SpinnerIcon class="animate-spin" /> {{ formatMessage(messages.loading) }}
			</p>
			<p v-else-if="error" class="m-0 text-sm text-red">{{ error }}</p>
			<div
				v-else-if="entry?.body.trim()"
				class="markdown-body text-sm"
				v-html="renderString(entry.body)"
			/>
			<p v-else class="m-0 text-sm text-secondary">{{ formatMessage(messages.empty) }}</p>
		</div>
		<footer class="terrarium-latest__foot">
			<Button type="transparent" @click="emit('openAll')">
				<HistoryIcon /> {{ formatMessage(messages.all) }}
			</Button>
		</footer>
	</section>
</template>

<style scoped lang="scss">
@use './terrarium-widget' as *;

.terrarium-latest {
	min-height: 0;
	overflow: hidden;
}

.terrarium-latest__meta {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	font-size: 0.8rem;
	color: var(--color-secondary);
}

.terrarium-latest__body {
	flex: 1 1 auto;
	min-height: 0;
	padding: 0 1.1rem;
	overflow-y: auto;
}

.terrarium-latest__foot {
	display: flex;
	justify-content: flex-end;
	padding: 0.35rem 0.6rem 0.6rem;
	border-top: 1px solid color-mix(in srgb, var(--color-contrast) 8%, transparent);
}
</style>
