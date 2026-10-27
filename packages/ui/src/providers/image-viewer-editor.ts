import type {
	ImageViewerEditorData,
	ImageViewerEditorSource,
} from '#ui/components/image-viewer-editor/types'

import { createContext } from '.'

export interface ImageViewerEditorContext {
	loadEditorData: (source: ImageViewerEditorSource) => Promise<ImageViewerEditorData>
	/**
	 * Remote image the webview cannot fetch() itself (no CORS headers / not in CSP
	 * connect-src, e.g. CurseForge CDN). The host app loads the bytes instead.
	 */
	loadRemoteImage?: (src: string) => Promise<ImageViewerEditorData | null>
	onShow?: () => void
	onHide?: () => void
}

export const [injectImageViewerEditor, provideImageViewerEditor] =
	createContext<ImageViewerEditorContext>('ImageViewerEditor')
