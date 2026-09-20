import { openUrl } from '@tauri-apps/plugin-opener'

/** Зовнішні посилання спільноти Terrarium — одне місце для всіх іконок у лаунчері. */
export const TERRARIUM_LINKS = {
	rules: 'https://terrarium-2.gitbook.io/terrarium-create/zagalna-informaciya/pravila-servera',
	discord: 'https://discord.com/invite/S7NUwdGQHZ',
	/** Вихідний код лаунчера — відкритий замість ліцензії */
	github: 'https://github.com/Kemzino/TerrariumLauncher',
} as const

/** Адреса ігрового сервера (SRV → порт резолвиться так само, як у грі). */
export const TERRARIUM_SERVER_ADDRESS = 'create.terrariumworlds.fun'

export function openTerrariumLink(key: keyof typeof TERRARIUM_LINKS) {
	void openUrl(TERRARIUM_LINKS[key])
}
