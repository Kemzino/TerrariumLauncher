import { computed, ref } from 'vue'

import {
	type Channel,
	type PackKind,
	type PackState,
	packStateKey,
	terrarium_get_state,
	terrarium_set_state,
	type TerrariumState,
} from '@/helpers/terrarium'

const emptyPack = (): PackState => ({ instance_id: null, installed_tag: null, excluded_paths: [] })

// Один спільний стан збірок на весь застосунок: налаштування зберігають ключ —
// головна бачить це одразу, без перемонтування.
const state = ref<TerrariumState>({
	admin_token: null,
	role: null,
	active_pack: 'client',
	active_channel: 'stable',
	client: emptyPack(),
	server: emptyPack(),
	client_test: emptyPack(),
	server_test: emptyPack(),
})
async function reload() {
	state.value = await terrarium_get_state()
}

async function save(next: TerrariumState) {
	await terrarium_set_state(next)
	state.value = next
}

// Перед кожним частковим записом перечитуємо файл: стан могли змінити ззовні
// (Rust після публікації, інший запущений лаунчер), і зберігати поверх нього
// копію з пам'яті — значить повернути, скажімо, щойно видалений адмін-ключ.
async function patch(changes: Partial<TerrariumState>) {
	await reload()
	await save({ ...state.value, ...changes })
}

async function patchPack(kind: PackKind, changes: Partial<PackState>, channel: Channel = 'stable') {
	await reload()
	const key = packStateKey(kind, channel)
	await save({ ...state.value, [key]: { ...state.value[key], ...changes } })
}

export function useTerrariumState() {
	const activePack = computed(() => state.value.active_pack)
	const activeChannel = computed<Channel>(() => state.value.active_channel ?? 'stable')
	const activePackState = computed(
		() => state.value[packStateKey(state.value.active_pack, activeChannel.value)],
	)
	const role = computed(() => state.value.role)
	const isAdmin = computed(() => state.value.role === 'admin')
	/** Адмін або тестер — бачить тестовий канал */
	const isTester = computed(() => state.value.role === 'admin' || state.value.role === 'tester')
	return {
		state,
		activePack,
		activeChannel,
		activePackState,
		role,
		isAdmin,
		isTester,
		reload,
		save,
		patch,
		patchPack,
	}
}
