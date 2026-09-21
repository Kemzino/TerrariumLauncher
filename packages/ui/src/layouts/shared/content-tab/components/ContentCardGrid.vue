<script setup lang="ts">
// Terrarium: щільна сітка плиток замість таблиці — для збірок на сотні модів.
// Той самий контракт (props/події/слоти), що в ContentCardTable, тож layout
// перемикає їх без змін у логіці вибору, оновлень чи перетягування між групами.
import {
	ArrowLeftRightIcon,
	DownloadIcon,
	LockIcon,
	MoreVerticalIcon,
	SpinnerIcon,
	TrashIcon,
	TriangleAlertIcon,
	UploadIcon,
} from '@modrinth/assets'
import { computed, getCurrentInstance, ref } from 'vue'

import AutoLink from '#ui/components/base/AutoLink.vue'
import Avatar from '#ui/components/base/Avatar.vue'
import { IconButton, TeleportOverflowMenu } from '#ui/components/base/buttons'
import Checkbox from '#ui/components/base/Checkbox.vue'
import Toggle from '#ui/components/base/Toggle.vue'
import { useVIntl } from '#ui/composables/i18n'
import { commonMessages } from '#ui/utils/common-messages'

import type { ContentCardTableItem } from '../types'

const { formatMessage } = useVIntl()

interface Props {
	items: ContentCardTableItem[]
	highlightedItemId?: string
	showSelection?: boolean
	hideDelete?: boolean
	/** Плитки можна тягнути (перенесення між групами) */
	draggable?: boolean
	// Приймаються для сумісності з ContentCardTable (layout перемикає компоненти
	// через <component :is>), у сітці не мають сенсу
	virtualized?: boolean
	hideHeader?: boolean
	flat?: boolean
}

const props = withDefaults(defineProps<Props>(), {
	highlightedItemId: undefined,
	showSelection: false,
	hideDelete: false,
	draggable: false,
	virtualized: false,
	hideHeader: false,
	flat: false,
})

const selectedIds = defineModel<string[]>('selectedIds', { default: () => [] })

const emit = defineEmits<{
	'update:enabled': [id: string, value: boolean]
	delete: [id: string, event: MouseEvent]
	update: [id: string]
	switchVersion: [id: string]
}>()

const instance = getCurrentInstance()
const hasDeleteListener = computed(() => typeof instance?.vnode.props?.onDelete === 'function')
const hasUpdateListener = computed(() => typeof instance?.vnode.props?.onUpdate === 'function')
const hasSwitchVersionListener = computed(
	() => typeof instance?.vnode.props?.onSwitchVersion === 'function',
)

const lastSelectedIndex = ref<number | null>(null)

function isItemSelected(id: string) {
	return selectedIds.value.includes(id)
}

function toggleItemSelection(id: string, selected: boolean, index: number, event?: MouseEvent) {
	if (selected && event?.shiftKey && lastSelectedIndex.value !== null) {
		const start = Math.min(lastSelectedIndex.value, index)
		const end = Math.max(lastSelectedIndex.value, index)
		const rangeIds = props.items
			.slice(start, end + 1)
			.filter((item) => !item.disabled)
			.map((item) => item.id)
		selectedIds.value = [...new Set([...selectedIds.value, ...rangeIds])]
	} else if (selected) {
		if (!selectedIds.value.includes(id)) selectedIds.value = [...selectedIds.value, id]
	} else {
		selectedIds.value = selectedIds.value.filter((x) => x !== id)
	}
	lastSelectedIndex.value = index
}

function clientWarningMessage(item: ContentCardTableItem) {
	switch (item.clientWarning) {
		case 'retained':
			return commonMessages.clientRetainedWarning
		case 'depends':
			return commonMessages.clientDependsWarning
		default:
			return commonMessages.clientOnlyWarning
	}
}

function isDisabled(item: ContentCardTableItem) {
	return !!item.disabled || !!item.installing
}

function actionTooltip(item: ContentCardTableItem, fallback: string) {
	return isDisabled(item) && item.disabledTooltip ? item.disabledTooltip : fallback
}

function isExternalLink(link: unknown) {
	return typeof link === 'string' && link.startsWith('http') ? '_blank' : undefined
}
</script>

<template>
	<div v-if="items.length > 0" class="content-grid" role="list">
		<div
			v-for="(item, index) in items"
			:key="item.id"
			role="listitem"
			:data-content-card-item="item.id"
			:data-drag-id="draggable && !item.disabled ? item.id : undefined"
			class="content-grid__tile flex min-w-0 items-center gap-2 rounded-xl border border-solid p-2 transition-colors"
			:class="[
				isItemSelected(item.id)
					? 'border-brand bg-brand-highlight'
					: 'border-surface-4 bg-surface-2',
				item.id === highlightedItemId ? 'outline outline-2 -outline-offset-2 outline-brand' : '',
				item.disabled && !item.installing ? 'opacity-50 grayscale' : '',
				item.installing ? 'opacity-50' : '',
			]"
		>
			<Checkbox
				v-if="showSelection"
				:model-value="isItemSelected(item.id)"
				:aria-label="item.project.title"
				:disabled="isDisabled(item)"
				class="shrink-0"
				@update:model-value="(value, event) => toggleItemSelection(item.id, value, index, event)"
			/>

			<div
				class="flex min-w-0 flex-1 items-center gap-2 transition-[filter,opacity] duration-200"
				:class="item.enabled === false && !item.disabled ? 'grayscale opacity-50' : ''"
			>
				<div class="relative flex shrink-0 items-center">
					<Avatar
						:src="item.project.icon_url"
						:alt="item.project.title"
						size="2.5rem"
						no-shadow
						class="rounded-xl border border-surface-5"
					/>
					<div
						v-if="item.installing"
						class="absolute inset-0 flex items-center justify-center rounded-xl bg-black/20"
					>
						<SpinnerIcon class="size-4 animate-spin text-white" />
					</div>
				</div>

				<div class="flex min-w-0 flex-1 flex-col" data-no-drag>
					<div class="flex min-w-0 items-center gap-1">
						<AutoLink
							v-tooltip="item.project.title"
							:target="isExternalLink(item.projectLink)"
							:to="item.projectLink"
							class="truncate text-sm font-semibold leading-5 text-contrast !decoration-contrast"
							:class="{ 'hover:underline': item.projectLink }"
						>
							{{ item.project.title }}
						</AutoLink>
						<span
							v-if="item.isClientOnly"
							v-tooltip="formatMessage(clientWarningMessage(item))"
							class="inline-flex size-4 shrink-0 cursor-help items-center justify-center"
							tabindex="0"
						>
							<TriangleAlertIcon class="pointer-events-none size-3.5 text-orange" />
						</span>
					</div>
					<div
						v-tooltip="item.version?.file_name"
						class="flex min-w-0 items-center gap-1 text-xs leading-4 text-secondary"
					>
						<template v-if="item.version && !item.external">
							<AutoLink
								:target="isExternalLink(item.versionLink)"
								:to="item.versionLink"
								class="truncate !decoration-secondary"
								:class="{ 'hover:underline': item.versionLink }"
							>
								{{ item.version.version_number }}
							</AutoLink>
						</template>
						<span v-else-if="item.external" class="flex items-center gap-1">
							<UploadIcon class="size-3 shrink-0" />
							<span class="truncate">{{ item.version?.file_name }}</span>
						</span>
						<span v-else-if="item.source" class="truncate">{{ item.source.project.title }}</span>
						<span v-else-if="item.owner" class="truncate">{{ item.owner.name }}</span>
					</div>
				</div>
			</div>

			<div class="flex shrink-0 items-center gap-0.5">
				<IconButton
					v-if="item.locked"
					v-tooltip="formatMessage(commonMessages.updateButton)"
					type="quiet"
					size="sm"
					:label="formatMessage(commonMessages.updateButton)"
					disabled
				>
					<LockIcon class="size-4" />
				</IconButton>
				<IconButton
					v-else-if="hasUpdateListener && item.hasUpdate"
					v-tooltip="actionTooltip(item, formatMessage(commonMessages.updateAvailableLabel))"
					type="quiet"
					size="sm"
					color="green"
					:label="actionTooltip(item, formatMessage(commonMessages.updateAvailableLabel))"
					:disabled="isDisabled(item)"
					class="hover:!bg-green focus-visible:!bg-green hover:!text-[var(--color-accent-contrast)] focus-visible:!text-[var(--color-accent-contrast)]"
					@click="emit('update', item.id)"
				>
					<DownloadIcon class="size-4" />
				</IconButton>
				<IconButton
					v-else-if="hasSwitchVersionListener && item.version && !item.hideSwitchVersion"
					v-tooltip="actionTooltip(item, formatMessage(commonMessages.switchVersionButton))"
					type="quiet"
					size="sm"
					:label="actionTooltip(item, formatMessage(commonMessages.switchVersionButton))"
					:disabled="isDisabled(item)"
					@click="emit('switchVersion', item.id)"
				>
					<ArrowLeftRightIcon class="size-4" />
				</IconButton>

				<Toggle
					v-if="item.enabled !== undefined && !item.hideToggle"
					v-tooltip="
						(isDisabled(item) || item.toggleDisabled) &&
						(item.toggleDisabledTooltip || item.disabledTooltip)
							? (item.toggleDisabledTooltip ?? item.disabledTooltip)
							: undefined
					"
					:model-value="item.enabled"
					:disabled="isDisabled(item) || item.toggleDisabled"
					:aria-label="item.project.title"
					class="mx-1 my-auto"
					@update:model-value="(val) => emit('update:enabled', item.id, val as boolean)"
				/>

				<IconButton
					v-if="hasDeleteListener && !hideDelete && !item.hideDelete"
					v-tooltip="actionTooltip(item, formatMessage(commonMessages.deleteLabel))"
					type="quiet"
					size="sm"
					:label="actionTooltip(item, formatMessage(commonMessages.deleteLabel))"
					:disabled="isDisabled(item)"
					@click="emit('delete', item.id, $event)"
				>
					<TrashIcon class="size-4 text-secondary" />
				</IconButton>

				<TeleportOverflowMenu
					v-if="item.overflowOptions?.length"
					type="quiet"
					size="sm"
					label="More options"
					:options="item.overflowOptions"
					:disabled="isDisabled(item)"
				>
					<MoreVerticalIcon class="size-4" />
				</TeleportOverflowMenu>
			</div>
		</div>
	</div>

	<div v-else class="flex items-center justify-center py-8">
		<slot name="empty">
			<span class="text-secondary">{{ formatMessage(commonMessages.noItemsLabel) }}</span>
		</slot>
	</div>
</template>

<style scoped>
.content-grid {
	display: grid;
	grid-template-columns: repeat(auto-fill, minmax(19rem, 1fr));
	gap: 0.5rem;
}
</style>
