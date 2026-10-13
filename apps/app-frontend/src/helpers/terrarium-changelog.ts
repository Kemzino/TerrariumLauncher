// Автоматичний список змін релізу: які моди додано, прибрано, оновлено (з
// версіями) та скільки змінилось конфігів. Додається до опису релізу, тож
// гравці бачать його у «Що нового», навіть якщо адмін нічого не написав.

import type { PackCore, PublishPreview } from '@/helpers/terrarium'

/** Що відомо про мод примірника (з тих самих даних, що й сторінка «Уміст»). */
export interface LocalModInfo {
	title: string
	projectId?: string
	version?: string
}

export interface ChangelogInput {
	preview: PublishPreview
	/** path → чи піде в реліз */
	selected: Record<string, boolean>
	/** Ім'я файлу (без `.disabled`) → інформація про мод */
	localMods: Map<string, LocalModInfo>
	/** Id проєкту Modrinth → назва (для модів, яких у примірнику вже нема) */
	projectTitles: Map<string, string>
}

const FOLDER_NAMES: Record<string, string> = {
	config: 'Конфіги',
	kubejs: 'KubeJS-скрипти',
	defaultconfigs: 'Стандартні конфіги світів',
	resourcepacks: 'Ресурспаки',
	shaderpacks: 'Шейдери',
	datapacks: 'Датапаки',
}

const SEPARATORS = '-_+ '

function fileName(path: string) {
	return path.slice(path.lastIndexOf('/') + 1)
}

/** Назва без версії: `sodium-neoforge-0.6.13+mc1.21.1.jar` → `sodium-neoforge`. */
export function modStem(file: string) {
	const base = file.replace(/\.(jar|zip)(\.disabled)?$/i, '').toLowerCase()
	const match = base.match(/^(.*?)[-_+ ]v?\d/)
	return (match ? match[1] : base).replace(/[-_ ]+(neoforge|forge|fabric|mc)$/, '')
}

/** Частини імен файлів, якими вони відрізняються: `1.0.188` → `1.0.194`. */
export function versionDiff(oldFile: string, newFile: string): [string, string] {
	let prefix = 0
	while (
		prefix < oldFile.length &&
		prefix < newFile.length &&
		oldFile[prefix] === newFile[prefix]
	) {
		prefix++
	}
	// Межа — після роздільника, щоб не різати версію посередині
	while (prefix > 0 && !SEPARATORS.includes(oldFile[prefix - 1])) prefix--
	let suffix = 0
	while (
		suffix < oldFile.length - prefix &&
		suffix < newFile.length - prefix &&
		oldFile[oldFile.length - 1 - suffix] === newFile[newFile.length - 1 - suffix]
	) {
		suffix++
	}
	const isBoundary = (s: string) =>
		s === '' || SEPARATORS.includes(s[0]) || /^\.(jar|zip)$/i.test(s)
	while (suffix > 0 && !isBoundary(oldFile.slice(oldFile.length - suffix))) suffix--
	return [
		oldFile.slice(prefix, oldFile.length - suffix),
		newFile.slice(prefix, newFile.length - suffix),
	]
}

function prettyStem(file: string) {
	const stem = modStem(file)
	return stem.charAt(0).toUpperCase() + stem.slice(1)
}

function coreLabel(core: PackCore) {
	const loader = core.loader.charAt(0).toUpperCase() + core.loader.slice(1)
	return core.loader === 'vanilla'
		? `Minecraft ${core.game_version}`
		: `Minecraft ${core.game_version} · ${loader} ${core.loader_version ?? ''}`.trim()
}

/** Id проєктів модів, що зникають зі збірки — щоб підтягнути їхні назви. */
export function removedProjectIds(preview: PublishPreview, selected: Record<string, boolean>) {
	const ids = new Set<string>()
	for (const path of droppedModPaths(preview, selected)) {
		const id = preview.release_projects?.[path]
		if (id) ids.add(id)
	}
	return [...ids]
}

/** Шляхи модів (у пакеті), які були в релізі, а в новий не підуть. */
function droppedModPaths(preview: PublishPreview, selected: Record<string, boolean>) {
	const paths = preview.removed_from_instance.filter((p) => p.startsWith('mods/'))
	for (const c of preview.candidates) {
		if (c.folder === 'mods' && c.in_release && !selected[c.path]) paths.push(c.pack_path)
	}
	return paths
}

/** Markdown зі списком змін; порожній рядок, якщо змін нема або це перший реліз. */
export function buildChangelog({ preview, selected, localMods, projectTitles }: ChangelogInput) {
	if (!preview.release_tag) return ''

	interface ModEntry {
		file: string
		title: string
		projectId?: string
		version?: string
	}
	const added: ModEntry[] = []
	const contentChanged: ModEntry[] = []
	for (const c of preview.candidates) {
		if (c.folder !== 'mods' || !selected[c.path]) continue
		const file = c.file_name.replace(/\.disabled$/, '')
		const info = localMods.get(file)
		const entry = {
			file,
			title: info?.title ?? prettyStem(file),
			projectId: info?.projectId,
			version: info?.version,
		}
		if (!c.in_release) added.push(entry)
		else if (c.changed) contentChanged.push(entry)
	}

	const removed: ModEntry[] = droppedModPaths(preview, selected).map((path) => {
		const file = fileName(path)
		const projectId = preview.release_projects?.[path]
		return {
			file,
			projectId,
			title: (projectId && projectTitles.get(projectId)) || prettyStem(file),
		}
	})

	// Новий файл того самого мода замість старого — це оновлення
	const updated: string[] = []
	for (let i = added.length - 1; i >= 0; i--) {
		const a = added[i]
		let j = a.projectId ? removed.findIndex((r) => r.projectId === a.projectId) : -1
		if (j < 0) j = removed.findIndex((r) => modStem(r.file) === modStem(a.file))
		if (j < 0) continue
		const old = removed.splice(j, 1)[0]
		// У релізі був вимкнений — гравці його не мали, тож це «додано»
		if (old.file.endsWith('.disabled')) continue
		const [from, to] = versionDiff(old.file, a.file)
		updated.push(from && to ? `- ${a.title}: ${from} → ${to}` : `- ${a.title}`)
		added.splice(i, 1)
	}
	for (const c of contentChanged) updated.push(`- ${c.title}`)
	// Вимкнений у релізі мод гравці й так не мали
	const removedActive = removed.filter((r) => !r.file.endsWith('.disabled'))

	const byTitle = (a: { title: string }, b: { title: string }) => a.title.localeCompare(b.title)
	const sections: string[] = []
	const core = preview.release_core
	if (
		core &&
		(core.game_version !== preview.core.game_version ||
			core.loader !== preview.core.loader ||
			core.loader_version !== preview.core.loader_version)
	) {
		sections.push(`**Ядро:** ${coreLabel(core)} → ${coreLabel(preview.core)}`)
	}
	if (added.length) {
		sections.push(
			'**Додано**\n' +
				added
					.sort(byTitle)
					.map((a) => (a.version ? `- ${a.title} (${a.version})` : `- ${a.title}`))
					.join('\n'),
		)
	}
	if (updated.length) {
		sections.push('**Оновлено**\n' + updated.sort((a, b) => a.localeCompare(b)).join('\n'))
	}
	if (removedActive.length) {
		sections.push(
			'**Прибрано**\n' +
				removedActive
					.sort(byTitle)
					.map((r) => `- ${r.title}`)
					.join('\n'),
		)
	}

	// Решта папок — лише підсумок, без переліку файлів
	const counts = new Map<string, { added: number; changed: number; removed: number }>()
	const count = (folder: string) => {
		let c = counts.get(folder)
		if (!c) counts.set(folder, (c = { added: 0, changed: 0, removed: 0 }))
		return c
	}
	for (const c of preview.candidates) {
		if (c.folder === 'mods' || c.folder.startsWith('.')) continue
		if (selected[c.path]) {
			if (!c.in_release) count(c.folder).added++
			else if (c.changed) count(c.folder).changed++
		} else if (c.in_release) {
			count(c.folder).removed++
		}
	}
	for (const path of preview.removed_from_instance) {
		const folder = path.split('/')[0]
		if (folder !== 'mods' && FOLDER_NAMES[folder]) count(folder).removed++
	}
	const other: string[] = []
	for (const [folder, c] of [...counts].sort(([a], [b]) => a.localeCompare(b))) {
		const parts = [
			c.changed && `змінено ${c.changed}`,
			c.added && `нових ${c.added}`,
			c.removed && `видалено ${c.removed}`,
		].filter(Boolean)
		if (parts.length) other.push(`- ${FOLDER_NAMES[folder] ?? folder}: ${parts.join(', ')}`)
	}
	if (other.length) sections.push('**Інше**\n' + other.join('\n'))

	return sections.join('\n\n')
}
