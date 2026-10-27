import {
	type ImageViewerEditorData,
	type ImageViewerEditorSource,
	provideImageViewerEditor,
} from '@modrinth/ui'
import { readFile } from '@tauri-apps/plugin-fs'
import { fetch as tauriFetch } from '@tauri-apps/plugin-http'

import { release_ads_window_hold, take_ads_window_hold } from '@/helpers/ads.js'

// Terrarium: CDN CurseForge не віддає CORS-заголовків, тож fetch() з WebView падає, і
// переглядач галереї проєкту CurseForge закривався одразу після відкриття. Такі картинки
// качаємо через Tauri HTTP (з Rust, без CORS); дозволені хости — у capabilities/plugins.json.
function isCurseForgeCdn(src: string) {
	try {
		const { protocol, hostname } = new URL(src)
		return (
			protocol === 'https:' && (hostname === 'forgecdn.net' || hostname.endsWith('.forgecdn.net'))
		)
	} catch {
		return false
	}
}

export function setupImageViewerEditorProvider() {
	provideImageViewerEditor({
		async loadEditorData(source: ImageViewerEditorSource): Promise<ImageViewerEditorData> {
			return {
				source: new Blob([await readFile(source.path)], { type: 'image/png' }),
			}
		},
		async loadRemoteImage(src: string): Promise<ImageViewerEditorData | null> {
			if (!isCurseForgeCdn(src)) return null
			const response = await tauriFetch(src)
			if (!response.ok) throw new Error(`Could not load image: ${response.status}`)
			return { source: await response.blob() }
		},
		onShow: take_ads_window_hold,
		onHide: release_ads_window_hold,
	})
}
