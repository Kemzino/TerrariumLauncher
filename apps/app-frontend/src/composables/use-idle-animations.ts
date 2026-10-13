import { getCurrentWindow } from '@tauri-apps/api/window'
import { computed, onScopeDispose, readonly, ref } from 'vue'

import { get_all as getRunningProcesses } from '@/helpers/process'
import type { AppEvents } from '@/providers/app-events'

import { useAppEvent } from './use-app-event'

// Анімації лаунчера (фон-слайдшоу під розмиттям, нескінченні підказки) змушують WebView
// перемальовувати вікно щокадру: ~0.85 ядра CPU, навіть коли лаунчер просто відкритий під
// грою. Поки вікно згорнуте / неактивне або йде гра, вони стоять на паузі: <html> отримує
// клас `terrarium-idle`, а компоненти з таймерами читають `animationsPaused`.

const hidden = ref(typeof document !== 'undefined' && document.hidden)
const focused = ref(typeof document === 'undefined' || document.hasFocus())
const gameRunning = ref(false)

const paused = computed(() => hidden.value || !focused.value || gameRunning.value)

/** true, поки анімації мають стояти. */
export const animationsPaused = readonly(paused)

export function useIdleAnimations(events: AppEvents) {
	const root = document.documentElement
	const apply = () => root.classList.toggle('terrarium-idle', paused.value)

	const onVisibility = () => {
		hidden.value = document.hidden
		apply()
	}
	const onFocus = () => {
		focused.value = true
		apply()
	}
	const onBlur = () => {
		focused.value = false
		apply()
	}
	document.addEventListener('visibilitychange', onVisibility)
	window.addEventListener('focus', onFocus)
	window.addEventListener('blur', onBlur)

	// Подія вікна Tauri надійніша за window.blur (наприклад, при Alt+Tab у повноекранну гру)
	let unlistenFocus: (() => void) | undefined
	getCurrentWindow()
		.onFocusChanged(({ payload }) => {
			focused.value = payload
			apply()
		})
		.then((unlisten) => (unlistenFocus = unlisten))
		.catch(() => {})

	const refreshProcesses = async () => {
		const processes = (await getRunningProcesses().catch(() => [])) as unknown[]
		gameRunning.value = processes.length > 0
		apply()
	}
	void refreshProcesses()
	useAppEvent('process', refreshProcesses, events)

	apply()

	onScopeDispose(() => {
		document.removeEventListener('visibilitychange', onVisibility)
		window.removeEventListener('focus', onFocus)
		window.removeEventListener('blur', onBlur)
		unlistenFocus?.()
		root.classList.remove('terrarium-idle')
	})
}
