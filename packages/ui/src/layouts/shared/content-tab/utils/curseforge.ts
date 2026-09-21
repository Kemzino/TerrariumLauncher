// Terrarium: моди з CurseForge (яких нема на Modrinth) — як показати їх у
// списку вмісту: іконка й назва з CF, посилання на сторінку мода в браузері,
// бейдж «CurseForge», оновлення веде на сторінку файлу (автозавантаження
// нема — більшість авторів на CF його забороняють).
import { CurseForgeIcon } from '@modrinth/assets'

import type { ContentCardTableItem, ContentItem } from '../types'

export const CURSEFORGE_SOURCE_ID = 'curseforge'

/** Мод без картки Modrinth, але впізнаний на CurseForge. */
export function isCurseForgeItem(item: ContentItem) {
	return !item.project && !!item.curseforge
}

/** Поля рядка таблиці, які CurseForge заміщає для такого мода. */
export function curseforgeTableFields(
	item: ContentItem,
	/** Примірник — щоб сторінка проєкту в лаунчері знала, куди ставити */
	instanceId?: string,
): Pick<
	ContentCardTableItem,
	'project' | 'projectLink' | 'version' | 'versionLink' | 'source' | 'external' | 'hasUpdate'
> | null {
	const cf = item.curseforge
	if (!cf || item.project) return null
	return {
		project: {
			id: `curseforge:${cf.mod_id}`,
			slug: null,
			title: cf.name,
			icon_url: cf.icon_url ?? item.embedded_metadata?.icon_url ?? null,
		},
		// Сторінка проєкту в лаунчері (та сама, що для Modrinth; id `cf-<modId>`)
		projectLink: {
			path: `/project/cf-${cf.mod_id}`,
			query: instanceId ? { i: instanceId } : {},
		},
		version: {
			id: `cf-file-${cf.file_id}`,
			version_number: item.embedded_metadata?.version ?? cf.display_name ?? cf.file_name,
			file_name: item.file_name,
		},
		versionLink: {
			path: `/project/cf-${cf.mod_id}/version/cf-file-${cf.file_id}`,
			query: instanceId ? { i: instanceId } : {},
		},
		source: {
			project: { id: CURSEFORGE_SOURCE_ID, slug: null, title: 'CurseForge', icon_url: null },
			icon: CurseForgeIcon,
			link: cf.url,
		},
		external: false,
		hasUpdate: !!cf.update && !item.locked,
	}
}
