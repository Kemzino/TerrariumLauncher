// Terrarium: CurseForge як повноцінне джерело — ті самі сторінки й компоненти,
// що для Modrinth. Тут обгортки Tauri-команд і мапери у формат Labrinth
// (Project / Version / TeamMember / SearchHit), щоб решта коду не розрізняла
// платформи; гравець бачить лише значок CurseForge.
import type { Labrinth } from '@modrinth/api-client'
import { invoke } from '@tauri-apps/api/core'

import type { CurseForgeFile } from '@/helpers/terrarium'

export const CURSEFORGE_ID_PREFIX = 'cf-'
const CURSEFORGE_VERSION_PREFIX = 'cf-file-'
const CURSEFORGE_USER_PREFIX = 'cf-user-'

export function isCurseForgeProjectId(id: string | number | null | undefined) {
	return typeof id === 'string' && id.startsWith(CURSEFORGE_ID_PREFIX) && !isCurseForgeVersionId(id)
}

export function isCurseForgeVersionId(id: string | null | undefined) {
	return typeof id === 'string' && id.startsWith(CURSEFORGE_VERSION_PREFIX)
}

export function curseforgeProjectId(modId: number) {
	return `${CURSEFORGE_ID_PREFIX}${modId}`
}

export function curseforgeModId(projectId: string): number {
	return Number(projectId.slice(CURSEFORGE_ID_PREFIX.length))
}

export function curseforgeVersionId(fileId: number) {
	return `${CURSEFORGE_VERSION_PREFIX}${fileId}`
}

export function curseforgeFileId(versionId: string): number {
	return Number(versionId.slice(CURSEFORGE_VERSION_PREFIX.length))
}

export function curseforgeModPageUrl(slug: string, projectType = 'mod') {
	const section =
		projectType === 'modpack'
			? 'modpacks'
			: projectType === 'resourcepack'
			? 'texture-packs'
			: projectType === 'shader'
				? 'shaders'
				: projectType === 'datapack'
					? 'data-packs'
					: 'mc-mods'
	return `https://www.curseforge.com/minecraft/${section}/${slug}`
}

// --- типи з app-lib

export interface CurseForgeCategory {
	id: number
	name: string
	slug: string
	icon_url?: string | null
	class_id?: number | null
}

export interface CurseForgeAuthor {
	id: number
	name: string
	url?: string | null
}

export interface CurseForgeScreenshot {
	id: number
	title: string
	description: string
	url: string
	thumbnail_url: string
}

export interface CurseForgeMod {
	id: number
	name: string
	slug: string
	summary: string
	class_id?: number | null
	links: {
		website_url?: string | null
		wiki_url?: string | null
		issues_url?: string | null
		source_url?: string | null
	}
	categories: CurseForgeCategory[]
	authors: CurseForgeAuthor[]
	icon_url?: string | null
	screenshots: CurseForgeScreenshot[]
	download_count: number
	thumbs_up_count: number
	date_created: string
	date_modified: string
	date_released: string
	latest_files_indexes: {
		game_version: string
		file_id: number
		file_name: string
		release_type: number
		mod_loader?: number | null
	}[]
	game_versions: string[]
	loaders: string[]
}

export interface CurseForgeFileDetails {
	id: number
	mod_id: number
	display_name: string
	file_name: string
	release_type: number
	file_date: string
	file_length: number
	download_count: number
	game_versions: string[]
	loaders: string[]
	downloadable: boolean
	sha1?: string | null
	dependencies: { mod_id: number; relation_type: number }[]
	url: string
}

export interface CurseForgeSearchParams {
	project_type?: string
	/** Шукати збірки (клас 4471) */
	modpacks?: boolean
	query?: string
	sort?: 'popularity' | 'downloads' | 'updated' | 'newest' | 'name'
	category_ids?: number[]
	game_version?: string
	loader?: string
	index?: number
	page_size?: number
}

export interface CurseForgeSearchResult {
	hits: CurseForgeMod[]
	total: number
	index: number
	page_size: number
}

export interface CurseForgeInstallResult {
	installed: string[]
	skipped: string[]
	failed: string[]
}

// --- команди

export function terrarium_curseforge_get_mod(modId: number, force = false) {
	return invoke<CurseForgeMod>('plugin:terrarium|terrarium_curseforge_get_mod', { modId, force })
}

export function terrarium_curseforge_get_description(modId: number) {
	return invoke<string>('plugin:terrarium|terrarium_curseforge_get_description', { modId })
}

export function terrarium_curseforge_get_changelog(modId: number, fileId: number) {
	return invoke<string>('plugin:terrarium|terrarium_curseforge_get_changelog', { modId, fileId })
}

export function terrarium_curseforge_get_files(
	modId: number,
	slug: string,
	options: { gameVersion?: string; loader?: string; index?: number } = {},
) {
	return invoke<{ files: CurseForgeFileDetails[]; total: number }>(
		'plugin:terrarium|terrarium_curseforge_get_files',
		{
			modId,
			slug,
			gameVersion: options.gameVersion ?? null,
			loader: options.loader ?? null,
			index: options.index ?? 0,
		},
	)
}

export function terrarium_curseforge_get_file(modId: number, fileId: number, slug: string) {
	return invoke<CurseForgeFileDetails>('plugin:terrarium|terrarium_curseforge_get_file', {
		modId,
		fileId,
		slug,
	})
}

/** `projectType: null` — категорії збірок */
export function terrarium_curseforge_get_categories(projectType: string | null = 'mod') {
	return invoke<CurseForgeCategory[]>('plugin:terrarium|terrarium_curseforge_get_categories', {
		projectType,
	})
}

export interface CurseForgeModpackPrepared {
	mrpack_path: string
	name: string
	version: string
	game_version: string
	loader?: string | null
	file_count: number
	/** Моди, для яких автор заборонив сторонні завантаження — збірка без них */
	missing: string[]
	icon_path?: string | null
}

/** Збірка CF → .mrpack у кеші лаунчера (далі — звичайна установка збірки) */
export function terrarium_curseforge_prepare_modpack(modId: number, fileId?: number | null) {
	return invoke<CurseForgeModpackPrepared>('plugin:terrarium|terrarium_curseforge_prepare_modpack', {
		modId,
		fileId: fileId ?? null,
	})
}

export function terrarium_curseforge_search(params: CurseForgeSearchParams) {
	return invoke<CurseForgeSearchResult>('plugin:terrarium|terrarium_curseforge_search', {
		params: {
			project_type: params.modpacks ? null : (params.project_type ?? 'mod'),
			modpacks: params.modpacks ?? false,
			query: params.query ?? null,
			sort: params.sort ?? null,
			category_ids: params.category_ids ?? [],
			game_version: params.game_version ?? null,
			loader: params.loader ?? null,
			index: params.index ?? 0,
			page_size: params.page_size ?? 20,
		},
	})
}

export function terrarium_curseforge_install(
	instanceId: string,
	modId: number,
	fileId?: number | null,
	withDependencies = true,
) {
	return invoke<CurseForgeInstallResult>('plugin:terrarium|terrarium_curseforge_install', {
		instanceId,
		modId,
		fileId: fileId ?? null,
		withDependencies,
	})
}

// --- мапери у формат Modrinth

function classProjectType(classId: number | null | undefined): string {
	switch (classId) {
		case 4471:
			return 'modpack'
		case 12:
			return 'resourcepack'
		case 6552:
			return 'shader'
		case 6945:
			return 'datapack'
		default:
			return 'mod'
	}
}

function releaseType(type: number): 'release' | 'beta' | 'alpha' {
	return type === 3 ? 'alpha' : type === 2 ? 'beta' : 'release'
}

/** Позначка, що проєкт із CurseForge (лежить у Project.terrarium_curseforge). */
export interface CurseForgeProjectMeta {
	mod_id: number
	slug: string
	url: string
	authors: CurseForgeAuthor[]
}

export type CurseForgeProject = Labrinth.Projects.v2.Project & {
	terrarium_curseforge: CurseForgeProjectMeta
}

export function getCurseForgeMeta(project: unknown): CurseForgeProjectMeta | null {
	const meta = (project as { terrarium_curseforge?: CurseForgeProjectMeta } | null)
		?.terrarium_curseforge
	return meta ?? null
}

export function cfModToProject(mod: CurseForgeMod, body = ''): CurseForgeProject {
	const projectType = classProjectType(mod.class_id)
	const url = mod.links.website_url ?? curseforgeModPageUrl(mod.slug, projectType)
	return {
		id: curseforgeProjectId(mod.id),
		slug: curseforgeProjectId(mod.id),
		project_type: projectType,
		team: curseforgeProjectId(mod.id),
		organization: null,
		title: mod.name,
		description: mod.summary,
		body,
		body_url: null,
		published: mod.date_created,
		updated: mod.date_modified,
		approved: mod.date_created,
		queued: null,
		status: 'approved',
		requested_status: null,
		moderator_message: null,
		license: { id: 'unknown', name: 'Невідома', url: null },
		client_side: 'unknown',
		server_side: 'unknown',
		downloads: mod.download_count,
		followers: mod.thumbs_up_count,
		categories: mod.categories.map((c) => c.name),
		additional_categories: [],
		game_versions: mod.game_versions,
		loaders: mod.loaders,
		versions: mod.latest_files_indexes.map((f) => curseforgeVersionId(f.file_id)),
		icon_url: mod.icon_url ?? null,
		issues_url: mod.links.issues_url ?? null,
		source_url: mod.links.source_url ?? null,
		wiki_url: mod.links.wiki_url ?? null,
		discord_url: null,
		donation_urls: [],
		gallery: mod.screenshots.map((s, index) => ({
			url: s.url,
			raw_url: s.url,
			featured: index === 0,
			title: s.title || null,
			description: s.description || null,
			created: mod.date_modified,
			ordering: index,
		})),
		color: null,
		thread_id: '',
		monetization_status: 'monetized',
		terrarium_curseforge: { mod_id: mod.id, slug: mod.slug, url, authors: mod.authors },
	} as unknown as CurseForgeProject
}

export function cfFileToVersion(
	file: CurseForgeFileDetails,
	mod: Pick<CurseForgeMod, 'id' | 'authors'>,
	changelog: string | null = null,
): Labrinth.Versions.v2.Version {
	return {
		id: curseforgeVersionId(file.id),
		project_id: curseforgeProjectId(mod.id),
		author_id: mod.authors[0] ? `${CURSEFORGE_USER_PREFIX}${mod.authors[0].id}` : '',
		featured: false,
		name: file.display_name || file.file_name,
		version_number: file.display_name || file.file_name,
		changelog,
		changelog_url: null,
		date_published: file.file_date,
		downloads: file.download_count,
		version_type: releaseType(file.release_type),
		status: 'listed',
		requested_status: null,
		files: [
			{
				hashes: { sha1: file.sha1 ?? '', sha512: '' },
				url: file.url,
				filename: file.file_name,
				primary: true,
				size: file.file_length,
				file_type: null,
			},
		],
		dependencies: file.dependencies
			.filter((d) => [1, 2, 3, 5].includes(d.relation_type))
			.map((d) => ({
				version_id: null,
				project_id: curseforgeProjectId(d.mod_id),
				file_name: null,
				dependency_type:
					d.relation_type === 3
						? 'required'
						: d.relation_type === 2
							? 'optional'
							: d.relation_type === 5
								? 'incompatible'
								: 'embedded',
			})),
		game_versions: file.game_versions,
		loaders: file.loaders,
	} as unknown as Labrinth.Versions.v2.Version
}

/** Версія з короткого індексу (без залежностей/чейнджлогу) — для установки й сумісності. */
export function cfFileIndexToVersion(
	index: CurseForgeMod['latest_files_indexes'][number],
	mod: CurseForgeMod,
): Labrinth.Versions.v2.Version {
	const loader =
		index.mod_loader === 1
			? 'forge'
			: index.mod_loader === 4
				? 'fabric'
				: index.mod_loader === 5
					? 'quilt'
					: index.mod_loader === 6
						? 'neoforge'
						: null
	return cfFileToVersion(
		{
			id: index.file_id,
			mod_id: mod.id,
			display_name: index.file_name,
			file_name: index.file_name,
			release_type: index.release_type,
			file_date: mod.date_modified,
			file_length: 0,
			download_count: 0,
			game_versions: [index.game_version],
			loaders: loader ? [loader] : [],
			downloadable: true,
			dependencies: [],
			url: `${curseforgeModPageUrl(mod.slug, classProjectType(mod.class_id))}/files/${index.file_id}`,
		},
		mod,
	)
}

/** Версії з коротких індексів: по одній на (версія гри, завантажувач). */
export function cfModToVersions(mod: CurseForgeMod): Labrinth.Versions.v2.Version[] {
	return mod.latest_files_indexes.map((index) => cfFileIndexToVersion(index, mod))
}

export function cfAuthorsToMembers(mod: CurseForgeMod): Labrinth.Teams.v2.TeamMember[] {
	return mod.authors.map((author, index) => ({
		team_id: curseforgeProjectId(mod.id),
		user: {
			id: `${CURSEFORGE_USER_PREFIX}${author.id}`,
			username: author.name,
			name: author.name,
			avatar_url: null,
			bio: null,
			created: mod.date_created,
			role: 'developer',
			badges: 0,
		},
		role: index === 0 ? 'Owner' : 'Author',
		permissions: 0,
		accepted: true,
		payouts_split: 0,
		ordering: index,
	})) as unknown as Labrinth.Teams.v2.TeamMember[]
}

/** Результат пошуку CF у формі картки пошуку Modrinth (v3). */
export function cfModToSearchHit(mod: CurseForgeMod) {
	const projectType = classProjectType(mod.class_id)
	return {
		project_id: curseforgeProjectId(mod.id),
		slug: curseforgeProjectId(mod.id),
		project_types: [projectType],
		project_type: projectType,
		name: mod.name,
		title: mod.name,
		summary: mod.summary,
		description: mod.summary,
		author: mod.authors[0]?.name ?? 'CurseForge',
		categories: mod.categories.map((c) => c.name),
		display_categories: mod.categories.slice(0, 3).map((c) => c.name),
		versions: mod.game_versions,
		game_versions: mod.game_versions,
		loaders: mod.loaders,
		downloads: mod.download_count,
		follows: mod.thumbs_up_count,
		icon_url: mod.icon_url ?? null,
		date_created: mod.date_created,
		date_modified: mod.date_modified,
		latest_version: mod.game_versions[0] ?? '',
		license: 'unknown',
		client_side: 'unknown',
		server_side: 'unknown',
		gallery: mod.screenshots.map((s) => s.thumbnail_url),
		featured_gallery: mod.screenshots[0]?.thumbnail_url ?? null,
		color: null,
		terrarium_curseforge: {
			mod_id: mod.id,
			slug: mod.slug,
			url: mod.links.website_url ?? curseforgeModPageUrl(mod.slug, projectType),
			authors: mod.authors,
		} satisfies CurseForgeProjectMeta,
	}
}

/** Файл із модалки версій → у формі CurseForgeFileDetails (для спільних маперів). */
export function cfFileFromListing(file: CurseForgeFile, modId: number): CurseForgeFileDetails {
	return {
		id: file.id,
		mod_id: modId,
		display_name: file.display_name,
		file_name: file.file_name,
		release_type: file.release_type,
		file_date: file.file_date,
		file_length: 0,
		download_count: 0,
		game_versions: file.game_versions.filter((v) => /^\d/.test(v)),
		loaders: [],
		downloadable: file.downloadable,
		dependencies: [],
		url: file.url,
	}
}

/** Усе для сторінки проєкту одним викликом. */
export async function loadCurseForgeProject(projectId: string, force = false) {
	const modId = curseforgeModId(projectId)
	const mod = await terrarium_curseforge_get_mod(modId, force)
	const [body, filesPage] = await Promise.all([
		terrarium_curseforge_get_description(modId).catch(() => ''),
		terrarium_curseforge_get_files(modId, mod.slug).catch(() => ({ files: [], total: 0 })),
	])
	const project = cfModToProject(mod, body)
	// Останні 50 файлів — повні; те, чого серед них нема, але є в індексах
	// останніх файлів за версією гри, — з індексів
	const known = new Set(filesPage.files.map((f) => f.id))
	const versions = [
		...filesPage.files.map((f) => cfFileToVersion(f, mod)),
		...mod.latest_files_indexes
			.filter((f) => !known.has(f.file_id))
			.map((f) => cfFileIndexToVersion(f, mod)),
	]
	project.versions = versions.map((v) => v.id)
	return { mod, project, versions, members: cfAuthorsToMembers(mod) }
}
