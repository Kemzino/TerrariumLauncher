import { getCurrentWindow } from '@tauri-apps/api/window'
import { computed, onScopeDispose, readonly, ref } from 'vue'

// Анімації лаунчера (фон-слайдшоу під розмиттям, нескінченні підказки) змушують WebView
// перемальовувати вікно щокадру: ~0.85 ядра CPU, навіть коли лаунчер просто відкритий під
// грою. Поки вікно згорнуте або неактивне, вони стоять на паузі: <html> отримує клас
// `terrarium-idle`, а компоненти з таймерами читають `animationsPaused`.
//
// Гра сама по собі анімації не зупиняє: коли гравець дивиться на лаунчер (вікно активне),
// фон рухається, а під грою лаунчер неактивний — і так стоїть.

const hidden = ref(typeof document !== 'undefined' && document.hidden)
// Спершу вважаємо вікно активним: document.hasFocus() у WebView2 на старті часто false, а
// подія фокуса могла прийти ще до підписки — тоді слайдшоу назавжди лишалось на паузі.
// Справжній стан нижче питаємо в Tauri.
const focused = ref(true)

// лише коли вікна не видно: фокус у WebView2 ненадійний, і через нього карусель застигала
// посеред наїзду, хоча гравець дивився на лаунчер
const paused = computed(() => hidden.value)

/** true, поки анімації мають стояти. */
export const animationsPaused = readonly(paused)

/**
 * true, поки вікна не видно (згорнуте / повністю закрите іншим вікном). Неактивне, але видиме
 * вікно сюди не входить: WebView2 часто помиляється з фокусом (старт, перетягування, другий
 * монітор), а слайдшоу через це назавжди застигало на одному кадрі.
 */
export const windowHidden = readonly(hidden)

export function useIdleAnimations() {
	const root = document.documentElement
	const appWindow = getCurrentWindow()
	let disposed = false
	const apply = () => root.classList.toggle('terrarium-idle', paused.value)
	const setFocused = (value: boolean) => {
		focused.value = value
		apply()
	}

	const onVisibility = () => {
		hidden.value = document.hidden
		apply()
	}
	// Вікно Tauri стартує невидимим (visible: false) — document.hidden, прочитаний при імпорті
	// модуля, міг лишитись true, якщо visibilitychange прийшов раніше за підписку. Тому
	// видимість перечитуємо і тут, і на кожному фокусі/взаємодії
	const syncHidden = () => {
		if (hidden.value !== document.hidden) {
			hidden.value = document.hidden
			apply()
		}
	}
	const onFocus = () => {
		syncHidden()
		setFocused(true)
	}
	const onBlur = () => setFocused(false)
	// Клік чи клавіша — вікно точно активне, навіть якщо подію фокуса пропустили. Рух миші
	// приходить і в неактивне вікно (другий монітор), тож тоді питаємо Tauri, не частіше
	// разу на секунду
	const onInteraction = () => {
		syncHidden()
		if (!focused.value) setFocused(true)
	}
	let lastFocusQuery = 0
	const onPointerMove = () => {
		const now = Date.now()
		if (now - lastFocusQuery < 1000) return
		lastFocusQuery = now
		syncHidden()
		appWindow
			.isFocused()
			.then((value) => {
				if (!disposed && value !== focused.value) setFocused(value)
			})
			.catch(() => {})
	}
	document.addEventListener('visibilitychange', onVisibility)
	window.addEventListener('focus', onFocus)
	window.addEventListener('blur', onBlur)
	window.addEventListener('pointermove', onPointerMove, { passive: true })
	window.addEventListener('pointerdown', onInteraction, { passive: true })
	window.addEventListener('keydown', onInteraction, { passive: true })

	// Подія вікна Tauri надійніша за window.blur (наприклад, при Alt+Tab у повноекранну гру)
	let unlistenFocus: (() => void) | undefined
	appWindow
		.onFocusChanged(({ payload }) => {
			syncHidden()
			setFocused(payload)
		})
		.then((unlisten) => {
			if (disposed) unlisten()
			else unlistenFocus = unlisten
			// Підписка вже є — тепер можна взяти поточний стан, нічого не пропустивши
			return appWindow.isFocused()
		})
		.then((value) => {
			if (!disposed && typeof value === 'boolean') setFocused(value)
		})
		.catch(() => {})

	hidden.value = document.hidden
	apply()

	onScopeDispose(() => {
		disposed = true
		document.removeEventListener('visibilitychange', onVisibility)
		window.removeEventListener('focus', onFocus)
		window.removeEventListener('blur', onBlur)
		window.removeEventListener('pointermove', onPointerMove)
		window.removeEventListener('pointerdown', onInteraction)
		window.removeEventListener('keydown', onInteraction)
		unlistenFocus?.()
		root.classList.remove('terrarium-idle')
	})
}
