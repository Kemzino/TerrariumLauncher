<template>
	<NewModal ref="modal" :header="formatMessage(messages.header)" max-width="820px">
		<div class="flex flex-col gap-3">
			<div v-if="loading" class="flex items-center gap-2 py-6 text-secondary">
				<SpinnerIcon class="animate-spin" />
				{{ formatMessage(messages.loading) }}
			</div>
			<p v-else-if="error" class="m-0 text-red">{{ error }}</p>
			<p v-else-if="reports.length === 0" class="m-0 text-secondary">
				{{ formatMessage(messages.empty) }}
			</p>
			<div v-else class="flex max-h-[65vh] flex-col gap-2 overflow-y-auto pr-1">
				<article
					v-for="report in reports"
					:key="report.number"
					class="rounded-xl border border-solid border-surface-5 bg-button-bg"
				>
					<button
						type="button"
						class="flex w-full cursor-pointer flex-wrap items-center gap-2 border-none bg-transparent p-3 text-left"
						@click="toggle(report.number)"
					>
						<ChevronRightIcon
							class="shrink-0 text-secondary transition-transform"
							:class="{ 'rotate-90': expanded.has(report.number) }"
						/>
						<span class="text-sm font-semibold text-contrast">{{ report.title }}</span>
						<span
							v-if="isUnread(report.number)"
							class="rounded-full bg-highlight-orange px-2 py-0.5 text-xs font-semibold text-orange"
						>
							{{ formatMessage(messages.unread) }}
						</span>
						<span
							v-if="!report.open"
							class="rounded-full bg-surface-4 px-2 py-0.5 text-xs font-semibold text-secondary"
						>
							{{ formatMessage(messages.closed) }}
						</span>
						<span class="ml-auto text-sm text-secondary">
							#{{ report.number }} · {{ formatDate(report.created_at) }}
						</span>
					</button>
					<div v-if="expanded.has(report.number)" class="px-4 pb-4">
						<div class="markdown-body text-sm" v-html="renderString(report.body)" />
						<div class="mt-3 flex justify-end">
							<Button type="outlined" @click="openUrl(report.html_url)">
								<ExternalIcon /> {{ formatMessage(messages.openIssue) }}
								<template v-if="report.comments > 0"> ({{ report.comments }})</template>
							</Button>
						</div>
					</div>
				</article>
			</div>
		</div>
		<template #actions>
			<div class="flex justify-end gap-2">
				<Button type="outlined" @click="openUrl(listUrl)"> <ExternalIcon /> GitHub </Button>
				<Button type="colored" color="brand" @click="modal?.hide()">
					<CheckIcon /> {{ formatMessage(commonMessages.closeButton) }}
				</Button>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
// Terrarium: репорти (баги, FPS, TPS), які гравці надсилають кнопкою в меню паузи
// (TerrariumWorld → issues з міткою performance-report). Непрочитані — новіші
// за останній переглянутий номер, він зберігається локально.
import { CheckIcon, ChevronRightIcon, ExternalIcon, SpinnerIcon } from '@modrinth/assets'
import { Button, commonMessages, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { renderString } from '@modrinth/utils'
import { openUrl } from '@tauri-apps/plugin-opener'
import { ref } from 'vue'

import {
	readReportsSeen,
	terrarium_list_perf_reports,
	type TerrariumPerfReport,
	writeReportsSeen,
} from '@/helpers/terrarium'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	header: { id: 'terrarium.reports.header', defaultMessage: 'Репорти гравців: баги, FPS, TPS' },
	loading: { id: 'terrarium.reports.loading', defaultMessage: 'Завантажую репорти…' },
	empty: { id: 'terrarium.reports.empty', defaultMessage: 'Репортів ще немає.' },
	unread: { id: 'terrarium.reports.unread', defaultMessage: 'Нове' },
	closed: { id: 'terrarium.reports.closed', defaultMessage: 'Закрито' },
	openIssue: { id: 'terrarium.reports.open-issue', defaultMessage: 'Відкрити на GitHub' },
})

const listUrl = 'https://github.com/Kemzino/TerrariumCreate/issues?q=label%3Aperformance-report'

const modal = ref<InstanceType<typeof NewModal>>()
const reports = ref<TerrariumPerfReport[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
const expanded = ref(new Set<number>())
/** Номер останнього репорту, який адмін уже бачив (на момент відкриття) */
const seenNumber = ref(0)

function isUnread(number: number) {
	return number > seenNumber.value
}

function toggle(number: number) {
	const next = new Set(expanded.value)
	if (next.has(number)) next.delete(number)
	else next.add(number)
	expanded.value = next
}

function formatDate(value: string) {
	const date = new Date(value)
	return Number.isNaN(date.getTime()) ? value : date.toLocaleString()
}

async function show() {
	modal.value?.show()
	seenNumber.value = readReportsSeen()
	expanded.value = new Set()
	loading.value = true
	error.value = null
	try {
		reports.value = await terrarium_list_perf_reports(50)
		const latest = reports.value.reduce((max, r) => Math.max(max, r.number), 0)
		if (latest > 0) {
			writeReportsSeen(latest)
			emit('seen', latest)
		}
	} catch (err) {
		error.value = String(err)
	} finally {
		loading.value = false
	}
}

const emit = defineEmits<{ seen: [number: number] }>()

defineExpose({ show })
</script>
