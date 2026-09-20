<script setup lang="ts">
import { ChevronLeftIcon, ChevronRightIcon, ExternalIcon, XIcon } from '@modrinth/assets'
import { Button, defineMessages, useVIntl } from '@modrinth/ui'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, onBeforeUnmount, ref, watch } from 'vue'

/** Повноекранний перегляд фото всередині лаунчера з гортанням. */
export interface LightboxImage {
	url: string
	name?: string
	caption?: string
}

const { formatMessage } = useVIntl()

const messages = defineMessages({
	close: { id: 'terrarium.lightbox.close', defaultMessage: 'Закрити' },
	prev: { id: 'terrarium.lightbox.prev', defaultMessage: 'Попереднє' },
	next: { id: 'terrarium.lightbox.next', defaultMessage: 'Наступне' },
	openExternal: { id: 'terrarium.lightbox.open', defaultMessage: 'Відкрити в браузері' },
	counter: { id: 'terrarium.lightbox.counter', defaultMessage: '{index} з {total}' },
})

const images = ref<LightboxImage[]>([])
const index = ref(0)
const open = ref(false)
const loaded = ref(false)

const current = computed(() => images.value[index.value] ?? null)
const hasMany = computed(() => images.value.length > 1)

function show(list: LightboxImage[], start = 0) {
	if (!list.length) return
	images.value = list
	index.value = Math.min(Math.max(start, 0), list.length - 1)
	loaded.value = false
	open.value = true
}

function hide() {
	open.value = false
}

function step(delta: number) {
	if (!hasMany.value) return
	loaded.value = false
	index.value = (index.value + delta + images.value.length) % images.value.length
}

function onKey(event: KeyboardEvent) {
	if (!open.value) return
	switch (event.key) {
		case 'Escape':
			hide()
			break
		case 'ArrowLeft':
			step(-1)
			break
		case 'ArrowRight':
			step(1)
			break
		default:
			return
	}
	event.preventDefault()
}

watch(open, (value) => {
	if (value) window.addEventListener('keydown', onKey)
	else window.removeEventListener('keydown', onKey)
})
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))

defineExpose({ show, hide })
</script>

<template>
	<Teleport to="body">
		<Transition name="terrarium-lightbox">
			<div
				v-if="open && current"
				class="terrarium-lightbox"
				role="dialog"
				aria-modal="true"
				@click.self="hide"
			>
				<div class="terrarium-lightbox__bar">
					<span v-if="hasMany" class="terrarium-lightbox__counter">
						{{ formatMessage(messages.counter, { index: index + 1, total: images.length }) }}
					</span>
					<span v-if="current.name" class="terrarium-lightbox__name">{{ current.name }}</span>
					<span class="flex-1" />
					<Button
						v-tooltip="formatMessage(messages.openExternal)"
						type="transparent"
						icon-only
						:aria-label="formatMessage(messages.openExternal)"
						@click="openUrl(current.url)"
					>
						<ExternalIcon />
					</Button>
					<Button
						v-tooltip="formatMessage(messages.close)"
						type="transparent"
						icon-only
						:aria-label="formatMessage(messages.close)"
						@click="hide"
					>
						<XIcon />
					</Button>
				</div>

				<button
					v-if="hasMany"
					type="button"
					class="terrarium-lightbox__nav terrarium-lightbox__nav--prev"
					:aria-label="formatMessage(messages.prev)"
					@click.stop="step(-1)"
				>
					<ChevronLeftIcon />
				</button>

				<img
					:key="current.url"
					:src="current.url"
					:alt="current.name ?? ''"
					class="terrarium-lightbox__img"
					:class="{ 'is-loaded': loaded }"
					@load="loaded = true"
					@click.stop
				/>

				<button
					v-if="hasMany"
					type="button"
					class="terrarium-lightbox__nav terrarium-lightbox__nav--next"
					:aria-label="formatMessage(messages.next)"
					@click.stop="step(1)"
				>
					<ChevronRightIcon />
				</button>

				<p v-if="current.caption" class="terrarium-lightbox__caption" @click.stop>
					{{ current.caption }}
				</p>
			</div>
		</Transition>
	</Teleport>
</template>

<style scoped lang="scss">
.terrarium-lightbox {
	position: fixed;
	inset: 0;
	z-index: 100;
	display: flex;
	align-items: center;
	justify-content: center;
	background: rgba(0, 0, 0, 0.88);
	backdrop-filter: blur(6px);
	cursor: zoom-out;
}

.terrarium-lightbox__bar {
	position: absolute;
	top: 0;
	left: 0;
	right: 0;
	display: flex;
	align-items: center;
	gap: 0.5rem;
	padding: 0.5rem 0.75rem;
	color: var(--color-contrast);
	background: linear-gradient(rgba(0, 0, 0, 0.6), transparent);
	cursor: default;
}

.terrarium-lightbox__counter {
	font-weight: 600;
}

.terrarium-lightbox__name {
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
	font-size: 0.85rem;
	color: var(--color-secondary);
}

.terrarium-lightbox__img {
	max-width: calc(100vw - 8rem);
	max-height: calc(100vh - 7rem);
	border-radius: var(--radius-md);
	box-shadow: 0 20px 60px rgba(0, 0, 0, 0.6);
	opacity: 0;
	transform: scale(0.97);
	transition:
		opacity 0.15s ease,
		transform 0.15s ease;
	cursor: default;

	&.is-loaded {
		opacity: 1;
		transform: scale(1);
	}
}

.terrarium-lightbox__nav {
	position: absolute;
	top: 50%;
	display: flex;
	align-items: center;
	justify-content: center;
	width: 3rem;
	height: 3rem;
	border: 0;
	border-radius: 999px;
	background: rgba(255, 255, 255, 0.12);
	color: #fff;
	cursor: pointer;
	transform: translateY(-50%);
	transition: background-color 0.12s ease;

	svg {
		width: 1.5rem;
		height: 1.5rem;
	}

	&:hover {
		background: rgba(255, 255, 255, 0.24);
	}

	&--prev {
		left: 1.25rem;
	}

	&--next {
		right: 1.25rem;
	}
}

.terrarium-lightbox__caption {
	position: absolute;
	bottom: 1rem;
	left: 50%;
	max-width: min(60rem, calc(100vw - 4rem));
	margin: 0;
	padding: 0.4rem 0.9rem;
	border-radius: 999px;
	background: rgba(0, 0, 0, 0.55);
	color: #fff;
	font-size: 0.9rem;
	transform: translateX(-50%);
	cursor: default;
}

.terrarium-lightbox-enter-active,
.terrarium-lightbox-leave-active {
	transition: opacity 0.15s ease;
}

.terrarium-lightbox-enter-from,
.terrarium-lightbox-leave-to {
	opacity: 0;
}
</style>
