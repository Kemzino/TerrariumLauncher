<script setup lang="ts">
import { BoxIcon, CheckIcon, ChevronDownIcon, LinkIcon, UnlinkIcon } from '@modrinth/assets'
import { Avatar, defineMessages, useVIntl } from '@modrinth/ui'
import { onBeforeUnmount, onMounted, ref } from 'vue'

import { getInstanceIconUrl } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types'

// Випадний список примірників для адміна: який примірник вважати збіркою.
// Показує іконку, назву, завантажувач і версію; дозволяє відв'язати.
const props = defineProps<{
	instances: GameInstance[]
	current: GameInstance | undefined
	disabled?: boolean
}>()
const emit = defineEmits<{ select: [instanceId: string]; unlink: [] }>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
	placeholder: { id: 'terrarium.picker.placeholder', defaultMessage: 'Прив’язати примірник…' },
	linked: { id: 'terrarium.picker.linked', defaultMessage: 'Примірник збірки' },
	unlink: { id: 'terrarium.picker.unlink', defaultMessage: 'Відв’язати примірник' },
	unlinkHint: {
		id: 'terrarium.picker.unlink-hint',
		defaultMessage: 'Примірник лишиться в бібліотеці',
	},
	empty: { id: 'terrarium.picker.empty', defaultMessage: 'Примірників ще немає' },
})

const open = ref(false)
const root = ref<HTMLElement | null>(null)

function toggle() {
	if (props.disabled) return
	open.value = !open.value
}

function choose(id: string) {
	open.value = false
	if (id !== props.current?.id) emit('select', id)
}

function unlink() {
	open.value = false
	emit('unlink')
}

function onDocumentClick(event: MouseEvent) {
	if (open.value && root.value && !root.value.contains(event.target as Node)) open.value = false
}

function onKey(event: KeyboardEvent) {
	if (event.key === 'Escape') open.value = false
}

onMounted(() => {
	document.addEventListener('mousedown', onDocumentClick)
	document.addEventListener('keydown', onKey)
})
onBeforeUnmount(() => {
	document.removeEventListener('mousedown', onDocumentClick)
	document.removeEventListener('keydown', onKey)
})

function loaderLabel(i: GameInstance) {
	const loader = i.loader ? i.loader.charAt(0).toUpperCase() + i.loader.slice(1) : ''
	return `${loader} ${i.game_version}`.trim()
}
</script>

<template>
	<div ref="root" class="instance-picker" :class="{ 'is-open': open, 'is-disabled': disabled }">
		<button type="button" class="instance-picker__chip" :disabled="disabled" @click="toggle">
			<template v-if="current">
				<Avatar
					size="1.75rem"
					:src="getInstanceIconUrl(current.icon_path)"
					:tint-by="current.id"
					pad-transparent-corners
					class="instance-picker__avatar"
				/>
				<span class="instance-picker__text">
					<span class="instance-picker__label">{{ formatMessage(messages.linked) }}</span>
					<span class="instance-picker__name">{{ current.name }}</span>
				</span>
			</template>
			<template v-else>
				<LinkIcon class="instance-picker__icon" />
				<span class="instance-picker__name">{{ formatMessage(messages.placeholder) }}</span>
			</template>
			<ChevronDownIcon class="instance-picker__chevron" />
		</button>

		<Transition name="instance-picker">
			<div v-if="open" class="instance-picker__menu" role="listbox">
				<p v-if="!instances.length" class="instance-picker__empty">
					{{ formatMessage(messages.empty) }}
				</p>
				<div class="instance-picker__list">
					<button
						v-for="i in instances"
						:key="i.id"
						type="button"
						role="option"
						class="instance-picker__item"
						:class="{ 'is-current': i.id === current?.id }"
						:aria-selected="i.id === current?.id"
						@click="choose(i.id)"
					>
						<Avatar
							size="2rem"
							:src="getInstanceIconUrl(i.icon_path)"
							:tint-by="i.id"
							pad-transparent-corners
							class="instance-picker__avatar"
						/>
						<span class="instance-picker__text">
							<span class="instance-picker__name">{{ i.name }}</span>
							<span class="instance-picker__meta">
								<BoxIcon /> {{ loaderLabel(i) }}
							</span>
						</span>
						<CheckIcon v-if="i.id === current?.id" class="instance-picker__check" />
					</button>
				</div>
				<button v-if="current" type="button" class="instance-picker__unlink" @click="unlink">
					<UnlinkIcon />
					<span class="instance-picker__text">
						<span class="instance-picker__name">{{ formatMessage(messages.unlink) }}</span>
						<span class="instance-picker__meta">{{ formatMessage(messages.unlinkHint) }}</span>
					</span>
				</button>
			</div>
		</Transition>
	</div>
</template>

<style scoped lang="scss">
.instance-picker {
	position: relative;
	display: inline-block;
}

.instance-picker__chip {
	display: inline-flex;
	align-items: center;
	gap: 0.6rem;
	padding: 0.4rem 0.75rem 0.4rem 0.5rem;
	border: 1px solid color-mix(in srgb, var(--color-contrast) 14%, transparent);
	border-radius: 9999px;
	background: var(--terrarium-glass);
	backdrop-filter: blur(10px);
	color: var(--color-contrast);
	font: inherit;
	text-align: left;
	cursor: pointer;
	transition:
		border-color 0.12s ease,
		background-color 0.12s ease;

	&:hover,
	.is-open & {
		border-color: var(--color-brand);
	}

	&:disabled {
		cursor: default;
		opacity: 0.6;
	}
}

.instance-picker__avatar {
	flex-shrink: 0;
	border-radius: var(--radius-sm);
}

.instance-picker__icon {
	width: 1.1rem;
	height: 1.1rem;
	margin-left: 0.25rem;
	color: var(--color-brand);
}

.instance-picker__text {
	display: flex;
	flex-direction: column;
	min-width: 0;
	line-height: 1.15;
}

.instance-picker__label {
	font-size: 0.65rem;
	font-weight: 700;
	letter-spacing: 0.08em;
	text-transform: uppercase;
	color: var(--color-secondary);
}

.instance-picker__name {
	font-size: 0.9rem;
	font-weight: 600;
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
	max-width: 18rem;
}

.instance-picker__meta {
	display: inline-flex;
	align-items: center;
	gap: 0.3rem;
	font-size: 0.75rem;
	color: var(--color-secondary);

	svg {
		width: 0.8rem;
		height: 0.8rem;
	}
}

.instance-picker__chevron {
	width: 1rem;
	height: 1rem;
	color: var(--color-secondary);
	transition: transform 0.15s ease;

	.is-open & {
		transform: rotate(180deg);
	}
}

.instance-picker__menu {
	position: absolute;
	z-index: 30;
	left: 0;
	bottom: calc(100% + 0.4rem);
	width: 22rem;
	max-width: calc(100vw - 4rem);
	padding: 0.4rem;
	border-radius: var(--radius-lg);
	background: var(--color-raised-bg);
	border: 1px solid var(--color-divider);
	box-shadow: 0 16px 40px rgba(0, 0, 0, 0.4);
}

.instance-picker__list {
	max-height: 18rem;
	overflow-y: auto;
	display: flex;
	flex-direction: column;
	gap: 0.1rem;
}

.instance-picker__empty {
	margin: 0.5rem 0.6rem;
	color: var(--color-secondary);
	font-size: 0.9rem;
}

.instance-picker__item,
.instance-picker__unlink {
	display: flex;
	align-items: center;
	gap: 0.6rem;
	width: 100%;
	padding: 0.4rem 0.5rem;
	border: 0;
	border-radius: var(--radius-md);
	background: transparent;
	color: var(--color-base);
	font: inherit;
	text-align: left;
	cursor: pointer;

	&:hover {
		background: var(--color-button-bg);
	}
}

.instance-picker__item.is-current {
	background: var(--color-brand-highlight);

	.instance-picker__name {
		color: var(--color-brand);
	}
}

.instance-picker__check {
	flex-shrink: 0;
	width: 1rem;
	height: 1rem;
	margin-left: auto;
	color: var(--color-brand);
}

.instance-picker__unlink {
	margin-top: 0.3rem;
	border-top: 1px solid var(--color-divider);
	border-radius: 0 0 var(--radius-md) var(--radius-md);
	padding-top: 0.6rem;

	> svg {
		width: 1.1rem;
		height: 1.1rem;
		margin: 0 0.45rem;
		color: var(--color-red);
	}

	&:hover .instance-picker__name {
		color: var(--color-red);
	}
}

.instance-picker-enter-active,
.instance-picker-leave-active {
	transition:
		opacity 0.12s ease,
		transform 0.12s ease;
}

.instance-picker-enter-from,
.instance-picker-leave-to {
	opacity: 0;
	transform: translateY(4px);
}
</style>
