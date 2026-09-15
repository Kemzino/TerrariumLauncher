<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'

// Накладний скролбар для контейнера `target`: не займає місця в макеті,
// з'являється під час прокрутки або при наведенні на правий край і згасає.
// Нативний скролбар контейнера має бути схований через CSS.
const props = withDefaults(defineProps<{ target: string; hideAfterMs?: number }>(), {
	hideAfterMs: 900,
})

const visible = ref(false)
const scrollable = ref(false)
const thumbTop = ref(0)
const thumbHeight = ref(0)
const dragging = ref(false)
const hovering = ref(false)

let el: HTMLElement | null = null
let hideTimer: ReturnType<typeof setTimeout> | null = null
let resizeObserver: ResizeObserver | null = null
let mutationObserver: MutationObserver | null = null
let dragStartY = 0
let dragStartScroll = 0

const MIN_THUMB = 28

function update() {
	if (!el) return
	const { scrollTop, scrollHeight, clientHeight } = el
	scrollable.value = scrollHeight > clientHeight + 1
	if (!scrollable.value) {
		visible.value = false
		return
	}
	const trackHeight = clientHeight
	const height = Math.max(MIN_THUMB, (clientHeight / scrollHeight) * trackHeight)
	const maxTop = trackHeight - height
	thumbHeight.value = height
	thumbTop.value = (scrollTop / (scrollHeight - clientHeight)) * maxTop
}

function reveal() {
	if (!scrollable.value) return
	visible.value = true
	if (hideTimer) clearTimeout(hideTimer)
	hideTimer = setTimeout(() => {
		if (!dragging.value && !hovering.value) visible.value = false
	}, props.hideAfterMs)
}

function onScroll() {
	update()
	reveal()
}

function onThumbDown(event: MouseEvent) {
	if (!el) return
	event.preventDefault()
	dragging.value = true
	dragStartY = event.clientY
	dragStartScroll = el.scrollTop
	window.addEventListener('mousemove', onDragMove)
	window.addEventListener('mouseup', onDragEnd)
}

function onDragMove(event: MouseEvent) {
	if (!el || !dragging.value) return
	const { scrollHeight, clientHeight } = el
	const trackHeight = clientHeight - thumbHeight.value
	if (trackHeight <= 0) return
	const ratio = (scrollHeight - clientHeight) / trackHeight
	el.scrollTop = dragStartScroll + (event.clientY - dragStartY) * ratio
}

function onDragEnd() {
	dragging.value = false
	window.removeEventListener('mousemove', onDragMove)
	window.removeEventListener('mouseup', onDragEnd)
	reveal()
}

function onTrackClick(event: MouseEvent) {
	if (!el || dragging.value) return
	const track = event.currentTarget as HTMLElement
	const y = event.clientY - track.getBoundingClientRect().top
	const { scrollHeight, clientHeight } = el
	const target = (y / clientHeight) * scrollHeight - clientHeight / 2
	el.scrollTo({ top: target, behavior: 'smooth' })
}

onMounted(() => {
	el = document.querySelector<HTMLElement>(props.target)
	if (!el) return
	el.addEventListener('scroll', onScroll, { passive: true })
	resizeObserver = new ResizeObserver(update)
	resizeObserver.observe(el)
	// Контент сторінок змінюється при навігації — стежимо за висотою
	mutationObserver = new MutationObserver(update)
	mutationObserver.observe(el, { childList: true, subtree: true })
	update()
})

onBeforeUnmount(() => {
	if (el) el.removeEventListener('scroll', onScroll)
	resizeObserver?.disconnect()
	mutationObserver?.disconnect()
	if (hideTimer) clearTimeout(hideTimer)
	onDragEnd()
})
</script>

<template>
	<div
		v-if="scrollable"
		class="terrarium-scrollbar"
		:class="{ 'is-visible': visible || hovering || dragging, 'is-dragging': dragging }"
		@mouseenter="((hovering = true), reveal())"
		@mouseleave="((hovering = false), reveal())"
		@click="onTrackClick"
	>
		<div
			class="terrarium-scrollbar__thumb"
			:style="{ top: `${thumbTop}px`, height: `${thumbHeight}px` }"
			@mousedown="onThumbDown"
			@click.stop
		></div>
	</div>
</template>

<style scoped lang="scss">
.terrarium-scrollbar {
	position: absolute;
	top: 0;
	right: 0;
	bottom: 0;
	width: 12px;
	z-index: 40;
	opacity: 0;
	transition: opacity 0.25s ease;
	pointer-events: auto;

	&.is-visible {
		opacity: 1;
	}
}

.terrarium-scrollbar__thumb {
	position: absolute;
	right: 3px;
	width: 6px;
	border-radius: 9999px;
	background: color-mix(in srgb, var(--color-contrast) 35%, transparent);
	transition:
		background-color 0.15s ease,
		width 0.15s ease;

	.terrarium-scrollbar:hover &,
	.is-dragging & {
		width: 8px;
		background: color-mix(in srgb, var(--color-brand) 85%, transparent);
	}
}
</style>
