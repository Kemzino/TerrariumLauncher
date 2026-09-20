import { invoke } from '@tauri-apps/api/core'
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

/** Стрічка новин: JSON, який GitHub Actions збирає з Discord (репо TerrariumNews). */
export const TERRARIUM_NEWS_URL =
	'https://raw.githubusercontent.com/Kemzino/TerrariumNews/main/news.json'

export function openTerrariumLink(key: keyof typeof TERRARIUM_LINKS) {
	void openDiscordLink(TERRARIUM_LINKS[key])
}

/**
 * Посилання Discord відкриваємо в застосунку (діплінк `discord://`), а якщо
 * його не встановлено — у браузері. Інші URL — як звичайно.
 */
export async function openDiscordLink(url: string) {
	if (!/^https:\/\/(discord|discordapp)\.com\//.test(url)) {
		await openUrl(url)
		return
	}
	try {
		await invoke<boolean>('plugin:terrarium|terrarium_open_discord', { url })
	} catch {
		await openUrl(url)
	}
}
