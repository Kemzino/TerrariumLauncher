<template>
	<Transition name="splash-fade" @after-leave="onAfterLeave">
		<div v-if="!doneLoading" class="splash-screen" :class="`${theme.active}-mode`">
			<div class="app-logo-wrapper" data-tauri-drag-region>
				<img :src="terrariumLogo" alt="Terrarium" class="app-logo" />
				<ProgressBar class="loading-bar" :progress="Math.min(loadingProgress, 100)" />
				<span v-if="message">{{ message }}</span>
			</div>
			<div class="powered-by" data-tauri-drag-region>
				<span>Launcher powered by</span>
				<ModrinthIcon class="powered-by__icon" />
				<span>Modrinth</span>
			</div>
			<div class="gradient-bg" data-tauri-drag-region></div>
			<div class="base-bg"></div>
		</div>
	</Transition>
</template>

<script setup>
import { ModrinthIcon } from '@modrinth/assets'
import { injectLoadingState } from '@modrinth/ui'
import { onMounted, ref, watch } from 'vue'

import terrariumLogo from '@/assets/terrarium/logo.png'
import ProgressBar from '@/components/ui/ProgressBar.vue'
import { useAppEvent } from '@/composables/use-app-event'
import { useTheme } from '@/composables/use-theme.ts'
import { debugStartup } from '@/helpers/startup-debug'

const theme = useTheme()

const doneLoading = ref(false)
const loadingProgress = ref(0)
const message = ref()

const MIN_DISPLAY_MS = 500
const mountedAt = Date.now()

const loading = injectLoadingState()
onMounted(() => debugStartup('Splash mounted'))

function onAfterLeave() {
	debugStartup('Splash fade completed', { displayedMs: Date.now() - mountedAt })
	loading.setEnabled(true)
}

watch(
	[loading.barEnabled, loading.pending],
	([barEnabled, pending]) => {
		debugStartup('Splash loading state changed', { barEnabled, pending })
		if (barEnabled) {
			return
		}

		if (pending) {
			loadingProgress.value = 0
			fakeLoadingIncrease()
			return
		}

		const elapsed = Date.now() - mountedAt
		const delay = Math.max(0, MIN_DISPLAY_MS - elapsed)
		debugStartup('Splash dismissal scheduled', { delayMs: delay, displayedMs: elapsed })

		setTimeout(() => {
			if (loading.pending.value) {
				debugStartup('Splash dismissal deferred: new loading work')
				return
			}
			doneLoading.value = true
			debugStartup('Splash fade started', { displayedMs: Date.now() - mountedAt })
		}, delay)
	},
	{ immediate: true },
)

function fakeLoadingIncrease() {
	if (loadingProgress.value < 95) {
		setTimeout(() => {
			loadingProgress.value += 2
			fakeLoadingIncrease()
		}, 5)
	}
}

useAppEvent('loading', (e) => {
	if (e.event.type === 'directory_move') {
		loadingProgress.value = 100 * (e.fraction ?? 1)
		message.value = 'Updating app directory...'
	}
})
</script>

<style scoped lang="scss">
.splash-screen {
	position: fixed;
	inset: 0;
	z-index: 10000;
}

.splash-fade-leave-active {
	transition: opacity 0.3s ease-in-out;
}

.splash-fade-leave-to {
	opacity: 0;
}

.app-logo-wrapper {
	position: absolute;
	height: 100vh;
	width: 100%;

	display: flex;
	flex-direction: column;
	justify-content: center;
	align-items: center;

	gap: 1.25rem;
	color: var(--color-contrast);

	z-index: 9998;
}

.app-logo {
	width: clamp(14rem, 30vw, 22rem);
	height: auto;
	filter: drop-shadow(0 10px 30px rgba(0, 0, 0, 0.45));
	-webkit-user-drag: none;
	animation: logo-in 0.6s ease-out both;
}

.loading-bar {
	max-width: 20rem;
}

.powered-by {
	position: absolute;
	left: 0;
	right: 0;
	bottom: 1.5rem;
	display: flex;
	align-items: center;
	justify-content: center;
	gap: 0.35rem;
	font-size: 0.8rem;
	font-weight: 500;
	color: var(--color-secondary);
	z-index: 9998;
}

.powered-by__icon {
	width: 1rem;
	height: 1rem;
	color: var(--color-brand);
}

.gradient-bg {
	position: absolute;
	height: 100vh;
	width: 100vw;
	background:
		radial-gradient(70% 60% at 20% 100%, var(--terrarium-yellow-soft) 0%, transparent 60%),
		radial-gradient(60% 50% at 90% 0%, rgba(31, 95, 214, 0.35) 0%, transparent 60%),
		linear-gradient(180deg, var(--splash-tint-top) 0%, var(--splash-tint-bottom) 97.29%);
	z-index: 9997;
}

.base-bg {
	position: absolute;
	top: 0;
	left: 0;
	width: 100%;
	height: 100%;
	background: var(--color-bg);
	z-index: 9995;
}

@keyframes logo-in {
	from {
		opacity: 0;
		transform: translateY(12px) scale(0.96);
	}
	to {
		opacity: 1;
		transform: none;
	}
}
</style>
