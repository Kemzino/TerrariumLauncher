import { invoke } from '@tauri-apps/api/core'

export type PackKind = 'client' | 'server'
/** Канал релізів: stable — публічний репо, для всіх; test — приватний репо, бачать адміни й тестери */
export type Channel = 'stable' | 'test'
export type AccessRole = 'admin' | 'tester'

export interface PackState {
	instance_id: string | null
	installed_tag: string | null
	excluded_paths: string[]
}

export interface TerrariumState {
	admin_token: string | null
	role: AccessRole | null
	active_pack: PackKind
	active_channel: Channel
	client: PackState
	server: PackState
	client_test: PackState
	server_test: PackState
}

export function packStateKey(pack: PackKind, channel: Channel) {
	return channel === 'test' ? (`${pack}_test` as const) : pack
}

export interface TerrariumRelease {
	pack: PackKind
	/** З якого каналу (репозиторію) взято реліз */
	channel: Channel
	repo: string
	id: number
	/** Тестовий реліз, якого ще нема в стабільному репо — можна «поширити для всіх» */
	prerelease: boolean
	html_url: string
	tag: string
	name: string
	body: string | null
	published_at: string | null
	mrpack_name: string
	mrpack_url: string
	mrpack_size: number
	mrpack_asset_id: number
}

export async function terrarium_get_state() {
	return await invoke<TerrariumState>('plugin:terrarium|terrarium_get_state')
}

export async function terrarium_set_state(state: TerrariumState) {
	return await invoke<void>('plugin:terrarium|terrarium_set_state', { state })
}

export async function terrarium_fetch_latest_release(pack: PackKind, channel: Channel = 'stable') {
	return await invoke<TerrariumRelease>('plugin:terrarium|terrarium_fetch_latest_release', {
		pack,
		channel,
	})
}

/** «Поширити для всіх»: реліз копіюється з приватного тест-репо у стабільний. */
export async function terrarium_promote_release(pack: PackKind, releaseId: number) {
	return await invoke<TerrariumRelease>('plugin:terrarium|terrarium_promote_release', {
		pack,
		releaseId,
	})
}

export async function terrarium_download_release(release: TerrariumRelease) {
	return await invoke<string>('plugin:terrarium|terrarium_download_release', { release })
}

export interface AdminInfo {
	login: string
	client_repo: string
	can_push_client: boolean
	server_repo: string
	can_push_server: boolean
	can_read_server: boolean
	client_test_repo: string
	server_test_repo: string
	/** Читає приватний тестовий репо → тестер */
	can_read_test: boolean
	role: AccessRole | null
}

export interface PublishRequest {
	pack: PackKind
	instance_id: string
	tag: string
	name: string
	body: string | null
	/** Спершу в тест (pre-release) */
	prerelease: boolean
	excluded: string[]
}

export interface PublishCandidate {
	path: string
	/** Плоский шлях у пакеті (`mods/x.jar`) — за ним порівнюємо з релізом */
	pack_path: string
	/** Група мода (`mods/<Група>/`), якщо є */
	group: string | null
	file_name: string
	folder: string
	size: number | null
	in_release: boolean
	changed: boolean
	excluded: boolean
	disabled: boolean
}

export interface PackCore {
	game_version: string
	loader: string
	loader_version: string | null
}

export interface PublishPreview {
	release_tag: string | null
	release_prerelease: boolean
	core: PackCore
	release_core: PackCore | null
	name: string
	release_name: string | null
	icon_changed: boolean
	candidates: PublishCandidate[]
	removed_from_instance: string[]
}

export interface PublishedRelease {
	pack: PackKind
	prerelease: boolean
	tag: string
	html_url: string
	asset_name: string
}

// ---------------------------------------------------------------------------
// Синхронізація між клієнтською та серверною збірками (адмін)

export type SyncKind = 'mod' | 'config'
export type SyncStatus = 'same' | 'differs' | 'client_only' | 'server_only'

/** Один файл у порівнянні: як він є в клієнта і на сервері */
export interface SyncFile {
	kind: SyncKind
	/** Мод — ім'я jar-а; конфіг — шлях від примірника (`config/jei/x.json`) */
	key: string
	name: string
	client_size: number | null
	server_size: number | null
	client_group: string | null
	server_group: string | null
	status: SyncStatus
}

export interface SyncSection {
	kind: SyncKind
	/** Мод — назва групи (`''` — без групи); конфіг — `config/<запис>` або корінь */
	key: string
	name: string
	files: SyncFile[]
}

export interface SyncPreview {
	sections: SyncSection[]
}

export interface SyncRequest {
	client_instance_id: string
	server_instance_id: string
	to_server: string[]
	to_client: string[]
}

export interface SyncResult {
	copied: number
}

export async function terrarium_sync_preview(clientInstanceId: string, serverInstanceId: string) {
	return await invoke<SyncPreview>('plugin:terrarium|terrarium_sync_preview', {
		clientInstanceId,
		serverInstanceId,
	})
}

export async function terrarium_sync_apply(request: SyncRequest) {
	return await invoke<SyncResult>('plugin:terrarium|terrarium_sync_apply', { request })
}

// ---------------------------------------------------------------------------
// Спільна тека даних із Modrinth App

export interface ModrinthAppInfo {
	path: string
	instances: number
	importable: number
	already_shared: boolean
}

export interface ModrinthImportResult {
	imported: number
	skipped: number
}

export async function terrarium_detect_modrinth_app() {
	return await invoke<ModrinthAppInfo | null>('plugin:terrarium|terrarium_detect_modrinth_app')
}

/** Записує custom_dir; застосовується після перезапуску лаунчера. */
export async function terrarium_use_modrinth_directory() {
	return await invoke<string>('plugin:terrarium|terrarium_use_modrinth_directory')
}

export async function terrarium_import_modrinth_instances() {
	return await invoke<ModrinthImportResult>('plugin:terrarium|terrarium_import_modrinth_instances')
}

export async function terrarium_verify_admin_token(token: string) {
	return await invoke<AdminInfo>('plugin:terrarium|terrarium_verify_admin_token', { token })
}

export async function terrarium_publish_release(request: PublishRequest) {
	return await invoke<PublishedRelease>('plugin:terrarium|terrarium_publish_release', { request })
}

export async function terrarium_publish_preview(pack: PackKind, instanceId: string) {
	return await invoke<PublishPreview>('plugin:terrarium|terrarium_publish_preview', {
		pack,
		instanceId,
	})
}

/** Наступний patch-тег після поточного: v1.2.3 → v1.2.4; якщо нічого — v1.0.0. */
export function suggestNextTag(current: string | null | undefined): string {
	const match = current?.match(/^(v?)(\d+)\.(\d+)\.(\d+)$/)
	if (!match) return 'v1.0.0'
	const [, prefix, major, minor, patch] = match
	return `${prefix || 'v'}${major}.${minor}.${Number(patch) + 1}`
}

/** Після встановлення/оновлення: застосувати іконку з пакета (`.terrarium/icon.*`). */
export async function terrarium_apply_pack_branding(instanceId: string) {
	return await invoke<boolean>('plugin:terrarium|terrarium_apply_pack_branding', { instanceId })
}

// ---------------------------------------------------------------------------
// Групи модів: справжні підпапки mods/<Група>/, на час гри розкладаються в корінь

export interface ContentGroup {
	/** Шлях групи `A/B/C` (без `.disabled`) */
	path: string
	/** Останній сегмент */
	name: string
	parent: string | null
	/** Файлів безпосередньо в цій папці */
	files: number
	/** `false` — ця папка або предок має суфікс `.disabled` */
	enabled: boolean
}

const DISABLED_SUFFIX = '.disabled'

/** `mods/A/B.disabled/x.jar` → `A/B` (шлях групи без суфіксів); `mods/x.jar` → null */
export function modGroupOf(filePath: string | undefined | null): string | null {
	if (!filePath) return null
	const parts = filePath.split('/')
	if (parts.length < 3 || parts[0] !== 'mods') return null
	return parts
		.slice(1, -1)
		.map((seg) => (seg.endsWith(DISABLED_SUFFIX) ? seg.slice(0, -DISABLED_SUFFIX.length) : seg))
		.join('/')
}

/** Останній сегмент шляху групи: `A/B` → `B` */
export function modGroupName(groupPath: string) {
	return groupPath.split('/').pop() ?? groupPath
}

/** Батьківська група: `A/B` → `A`, `A` → null */
export function modGroupParent(groupPath: string): string | null {
	const i = groupPath.lastIndexOf('/')
	return i === -1 ? null : groupPath.slice(0, i)
}

/** Перед оновленням збірки в наявний примірник. */
export async function terrarium_prepare_pack_update(instanceId: string) {
	return await invoke<void>('plugin:terrarium|terrarium_prepare_pack_update', { instanceId })
}

export async function terrarium_list_mod_groups(instanceId: string) {
	return await invoke<ContentGroup[]>('plugin:terrarium|terrarium_list_mod_groups', { instanceId })
}

export async function terrarium_create_mod_group(instanceId: string, name: string) {
	return await invoke<string>('plugin:terrarium|terrarium_create_mod_group', { instanceId, name })
}

export async function terrarium_rename_mod_group(
	instanceId: string,
	oldName: string,
	newName: string,
) {
	return await invoke<string>('plugin:terrarium|terrarium_rename_mod_group', {
		instanceId,
		oldName,
		newName,
	})
}

export async function terrarium_delete_mod_group(instanceId: string, name: string) {
	return await invoke<void>('plugin:terrarium|terrarium_delete_mod_group', { instanceId, name })
}

/** Перемістити файли в групу (null — прибрати з групи). Повертає нові шляхи. */
export async function terrarium_set_mod_group_enabled(
	instanceId: string,
	name: string,
	enabled: boolean,
) {
	return await invoke<void>('plugin:terrarium|terrarium_set_mod_group_enabled', {
		instanceId,
		name,
		enabled,
	})
}

export async function terrarium_set_mod_group(
	instanceId: string,
	paths: string[],
	group: string | null,
) {
	return await invoke<string[]>('plugin:terrarium|terrarium_set_mod_group', {
		instanceId,
		paths,
		group,
	})
}
