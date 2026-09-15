import io, sys
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

# --- Hero: спільний стан замість локального ref ---
p = 'apps/app-frontend/src/components/ui/TerrariumHero.vue'
s = open(p, encoding='utf-8').read()
s = s.replace("import { useAppEvent } from '@/composables/use-app-event'\n",
              "import { useAppEvent } from '@/composables/use-app-event'\nimport { useTerrariumState } from '@/composables/use-terrarium-state'\n", 1)
s = s.replace("""import {
	type PublishedRelease,
	terrarium_download_release,
	terrarium_fetch_latest_release,
	terrarium_get_state,
	terrarium_set_state,
	type TerrariumRelease,
	type TerrariumState,
} from '@/helpers/terrarium'
""", """import {
	type PublishedRelease,
	terrarium_download_release,
	terrarium_fetch_latest_release,
	type TerrariumRelease,
} from '@/helpers/terrarium'
""", 1)
s = s.replace("const state = ref<TerrariumState>({ instance_id: null, installed_tag: null, admin_token: null })\n",
              "const { state, reload: reloadState, patch: patchState } = useTerrariumState()\n", 1)
s = s.replace("\t\tstate.value = await terrarium_get_state()\n\t\trelease.value = await terrarium_fetch_latest_release()\n",
              "\t\tawait reloadState()\n\t\trelease.value = await terrarium_fetch_latest_release()\n", 1)
s = s.replace("""			state.value = { instance_id: finished.instance_id, installed_tag: release.value!.tag }
			await terrarium_set_state(state.value)
""", """			await patchState({ instance_id: finished.instance_id, installed_tag: release.value!.tag })
""", 1)
s = s.replace("""			state.value = { ...state.value, installed_tag: release.value!.tag }
			await terrarium_set_state(state.value)
""", """			await patchState({ installed_tag: release.value!.tag })
""", 1)
s = s.replace("""	state.value = {
		...state.value,
		instance_id: linkTarget.value,
		installed_tag: release.value?.tag ?? null,
	}
	await terrarium_set_state(state.value).catch(handleError)
""", """	await patchState({
		instance_id: linkTarget.value,
		installed_tag: release.value?.tag ?? null,
	}).catch(handleError)
""", 1)
s = s.replace("""	state.value = { ...state.value, installed_tag: published.tag }
	release.value""", """	void patchState({ installed_tag: published.tag }).catch(handleError)
	release.value""", 1)
assert 'terrarium_set_state' not in s and 'terrarium_get_state' not in s, "лишились прямі виклики"
open(p, 'w', encoding='utf-8', newline='\n').write(s)

# --- Settings: зберігаємо через спільний стан ---
p = 'apps/app-frontend/src/components/ui/settings/terrarium/TerrariumSettings.vue'
s = open(p, encoding='utf-8').read()
s = s.replace("""import {
	type AdminInfo,
	terrarium_get_state,
	terrarium_set_state,
	terrarium_verify_admin_token,
} from '@/helpers/terrarium'
""", """import { useTerrariumState } from '@/composables/use-terrarium-state'
import { type AdminInfo, terrarium_verify_admin_token } from '@/helpers/terrarium'
""", 1)
s = s.replace("const { handleError, addNotification } = injectNotificationManager()\n",
              "const { handleError, addNotification } = injectNotificationManager()\nconst { state, reload: reloadState, patch: patchState } = useTerrariumState()\n", 1)
s = s.replace("""async function load() {
	const state = await terrarium_get_state()
	hasToken.value = !!state.admin_token
	if (state.admin_token) {
		try {
			info.value = await terrarium_verify_admin_token(state.admin_token)
""", """async function load() {
	await reloadState()
	hasToken.value = !!state.value.admin_token
	if (state.value.admin_token) {
		try {
			info.value = await terrarium_verify_admin_token(state.value.admin_token)
""", 1)
s = s.replace("""		const result = await terrarium_verify_admin_token(token)
		const state = await terrarium_get_state()
		await terrarium_set_state({ ...state, admin_token: token })
""", """		const result = await terrarium_verify_admin_token(token)
		await patchState({ admin_token: token })
""", 1)
s = s.replace("""		const state = await terrarium_get_state()
		await terrarium_set_state({ ...state, admin_token: null })
""", """		await patchState({ admin_token: null })
""", 1)
assert 'terrarium_set_state' not in s and 'terrarium_get_state' not in s
open(p, 'w', encoding='utf-8', newline='\n').write(s)
print("ok")
