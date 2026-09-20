<script setup lang="ts">
import { CheckCircleIcon, KeyIcon, SpinnerIcon, TrashIcon, XIcon } from '@modrinth/assets'
import { Button, defineMessages, injectNotificationManager, Input, useVIntl } from '@modrinth/ui'
import { onMounted, ref } from 'vue'

import TerrariumModrinthCard from '@/components/ui/TerrariumModrinthCard.vue'
import { useTerrariumState } from '@/composables/use-terrarium-state'
import { type AdminInfo, terrarium_verify_admin_token } from '@/helpers/terrarium'

const { formatMessage } = useVIntl()
const { handleError, addNotification } = injectNotificationManager()
const { state, reload: reloadState, patch: patchState } = useTerrariumState()

const messages = defineMessages({
	title: { id: 'terrarium.settings.title', defaultMessage: 'Адміністрування збірки' },
	description: {
		id: 'terrarium.settings.description',
		defaultMessage:
			'Адмін-ключ дає змогу редагувати збірку й публікувати оновлення на GitHub прямо з лаунчера. Ключ видає власник збірки.',
	},
	keyLabel: { id: 'terrarium.settings.key-label', defaultMessage: 'Ключ доступу' },
	noAccessTitle: { id: 'terrarium.settings.no-access', defaultMessage: 'Ключ не дає доступу' },
	invalidToken: {
		id: 'terrarium.settings.invalid-token',
		defaultMessage:
			'Збережений ключ GitHub більше не приймає (відкликано або перевидано). Встав новий.',
	},
	testerAs: {
		id: 'terrarium.settings.tester-as',
		defaultMessage: 'Режим тестера увімкнено: {login}',
	},
	testerHint: {
		id: 'terrarium.settings.tester-hint',
		defaultMessage:
			'Ти бачиш тестові версії збірки (канал «Тест» на головній) і можеш їх ставити окремим примірником.',
	},
	testRepos: { id: 'terrarium.settings.test-repos', defaultMessage: 'Тестові версії' },
	testReposHint: {
		id: 'terrarium.settings.test-repos-hint',
		defaultMessage:
			'Приватні репозиторії — бету бачать лише ті, кому дано доступ. Щоб зробити когось тестером: додай його як collaborator (Read) у тестовий репозиторій, він створює ключ GitHub з правом читання Contents і вставляє його тут.',
	},
	keyPlaceholder: {
		id: 'terrarium.settings.key-placeholder',
		defaultMessage: 'github_pat_… або ghp_…',
	},
	verifyAndSave: { id: 'terrarium.settings.verify', defaultMessage: 'Перевірити й зберегти' },
	remove: { id: 'terrarium.settings.remove', defaultMessage: 'Видалити ключ' },
	activeAs: {
		id: 'terrarium.settings.active-as',
		defaultMessage: 'Адмін-режим увімкнено: {login}',
	},
	noPush: {
		id: 'terrarium.settings.no-push',
		defaultMessage:
			'Ключ дійсний ({login}), але не має доступу ні до запису в репозиторії збірок, ні до читання серверного. Попроси адміна видати ключ.',
	},
	repoOk: { id: 'terrarium.settings.repo-ok', defaultMessage: 'запис' },
	repoNo: { id: 'terrarium.settings.repo-no', defaultMessage: 'лише читання' },
	clientPack: { id: 'terrarium.settings.client-pack', defaultMessage: 'Клієнтська збірка' },
	serverPack: { id: 'terrarium.settings.server-pack', defaultMessage: 'Серверна збірка' },
	saved: { id: 'terrarium.settings.saved', defaultMessage: 'Ключ збережено' },
	removed: { id: 'terrarium.settings.removed', defaultMessage: 'Ключ видалено' },
	notSet: {
		id: 'terrarium.settings.not-set',
		defaultMessage: 'Ключ не задано — ти граєш як звичайний гравець.',
	},
})

const tokenInput = ref('')
const hasToken = ref(false)
const info = ref<AdminInfo | null>(null)
const busy = ref(false)

async function load() {
	await reloadState()
	hasToken.value = !!state.value.admin_token
	if (state.value.admin_token) {
		try {
			info.value = await terrarium_verify_admin_token(state.value.admin_token)
			// Права могли змінитися на GitHub (перевидали токен, урізали доступ) —
			// оновлюємо збережену роль, щоб головна не показувала зайве
			if (info.value.role !== state.value.role) {
				await patchState({
					role: info.value.role,
					active_channel: info.value.role ? state.value.active_channel : 'stable',
				})
			}
		} catch (err) {
			info.value = null
			// 401 — токен відкликано; тримати роль далі нема сенсу
			if (String(err).includes('401')) {
				await patchState({ role: null, active_channel: 'stable' })
			}
		}
	}
}

async function verifyAndSave() {
	const token = tokenInput.value.trim()
	if (!token) return
	busy.value = true
	try {
		const result = await terrarium_verify_admin_token(token)
		if (!result.role) {
			addNotification({
				type: 'warning',
				title: formatMessage(messages.noAccessTitle),
				text: formatMessage(messages.noPush, { login: result.login }),
			})
		}
		// Ключ без ролі не зберігаємо — нема сенсу
		await patchState({
			admin_token: result.role ? token : null,
			role: result.role,
			active_channel: result.role ? state.value.active_channel : 'stable',
		})
		info.value = result
		hasToken.value = true
		tokenInput.value = ''
		addNotification({ type: 'success', title: formatMessage(messages.saved) })
	} catch (err) {
		handleError(err)
	} finally {
		busy.value = false
	}
}

async function remove() {
	busy.value = true
	try {
		await patchState({ admin_token: null, role: null, active_channel: 'stable' })
		info.value = null
		hasToken.value = false
		addNotification({ type: 'success', title: formatMessage(messages.removed) })
	} catch (err) {
		handleError(err)
	} finally {
		busy.value = false
	}
}

onMounted(load)
</script>

<template>
	<section>
		<h2 class="m-0 text-xl font-semibold text-contrast">{{ formatMessage(messages.title) }}</h2>
		<p class="m-0 mt-1">{{ formatMessage(messages.description) }}</p>

		<div class="mt-4 rounded-xl border border-solid border-surface-5 bg-button-bg p-4">
			<template v-if="info && (info.can_push_client || info.can_push_server)">
				<p class="m-0 flex items-center gap-2 font-medium text-contrast">
					<CheckCircleIcon class="shrink-0 text-brand" />
					{{ formatMessage(messages.activeAs, { login: info.login }) }}
				</p>
				<ul class="m-0 mt-2 list-none p-0 text-sm text-secondary">
					<li>
						{{ formatMessage(messages.clientPack) }} · <code>{{ info.client_repo }}</code> ·
						<span :class="info.can_push_client ? 'text-brand' : 'text-orange'">
							{{
								info.can_push_client
									? formatMessage(messages.repoOk)
									: formatMessage(messages.repoNo)
							}}
						</span>
					</li>
					<li>
						{{ formatMessage(messages.serverPack) }} · <code>{{ info.server_repo }}</code> ·
						<span :class="info.can_push_server ? 'text-brand' : 'text-orange'">
							{{
								info.can_push_server
									? formatMessage(messages.repoOk)
									: formatMessage(messages.repoNo)
							}}
						</span>
					</li>
					<li>
						{{ formatMessage(messages.testRepos) }} · <code>{{ info.client_test_repo }}</code> ·
						<code>{{ info.server_test_repo }}</code>
					</li>
				</ul>
				<p class="m-0 mt-2 text-sm text-secondary">{{ formatMessage(messages.testReposHint) }}</p>
			</template>
			<template v-else-if="info && info.role === 'tester'">
				<p class="m-0 flex items-center gap-2 font-medium text-contrast">
					<CheckCircleIcon class="shrink-0 text-brand" />
					{{ formatMessage(messages.testerAs, { login: info.login }) }}
				</p>
				<p class="m-0 mt-2 text-sm text-secondary">{{ formatMessage(messages.testerHint) }}</p>
			</template>
			<p v-else-if="info" class="m-0 flex items-center gap-2 text-orange">
				<XIcon class="shrink-0" />
				{{ formatMessage(messages.noPush, { login: info.login }) }}
			</p>
			<p v-else-if="hasToken" class="m-0 flex items-center gap-2 text-orange">
				<XIcon class="shrink-0" />
				{{ formatMessage(messages.invalidToken) }}
			</p>
			<p v-else class="m-0 text-secondary">{{ formatMessage(messages.notSet) }}</p>
		</div>

		<div class="mt-4 flex flex-col gap-2">
			<label class="font-semibold text-contrast" for="terrarium-admin-key">
				{{ formatMessage(messages.keyLabel) }}
			</label>
			<div class="flex flex-wrap items-center gap-2">
				<Input
					id="terrarium-admin-key"
					v-model="tokenInput"
					type="password"
					:placeholder="formatMessage(messages.keyPlaceholder)"
					autocomplete="off"
					:spellcheck="false"
					wrapper-class="flex-1 min-w-[16rem]"
					@keyup.enter="verifyAndSave"
				/>
				<Button
					type="colored"
					color="brand"
					:disabled="busy || !tokenInput.trim()"
					@click="verifyAndSave"
				>
					<SpinnerIcon v-if="busy" class="animate-spin" />
					<KeyIcon v-else />
					{{ formatMessage(messages.verifyAndSave) }}
				</Button>
				<Button v-if="hasToken" type="outlined" color="red" :disabled="busy" @click="remove">
					<TrashIcon />
					{{ formatMessage(messages.remove) }}
				</Button>
			</div>
		</div>

		<TerrariumModrinthCard class="mt-6" />
	</section>
</template>
