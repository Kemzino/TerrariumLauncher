<template>
	<NewModal ref="modal" :header="header" max-width="420px">
		<form class="flex flex-col gap-3" @submit.prevent="submit">
			<Input
				ref="input"
				v-model="name"
				type="text"
				autocomplete="off"
				:spellcheck="false"
				size="medium"
				:placeholder="formatMessage(messages.placeholder)"
				:maxlength="64"
			/>
			<span class="text-sm text-secondary">{{ formatMessage(messages.hint) }}</span>
		</form>

		<template #actions>
			<div class="flex justify-end gap-2">
				<Button type="outlined" @click="modal?.hide()">
					<XIcon />
					{{ formatMessage(commonMessages.cancelButton) }}
				</Button>
				<Button type="colored" color="brand" :disabled="!name.trim() || busy" @click="submit">
					<SpinnerIcon v-if="busy" class="animate-spin" />
					<CheckIcon v-else />
					{{ formatMessage(mode === 'rename' ? commonMessages.saveButton : messages.create) }}
				</Button>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import { CheckIcon, SpinnerIcon, XIcon } from '@modrinth/assets'
import { computed, nextTick, ref } from 'vue'

import { Button } from '#ui/components/base/buttons'
import Input from '#ui/components/base/inputs/Input.vue'
import NewModal from '#ui/components/modal/NewModal.vue'
import { defineMessages, useVIntl } from '#ui/composables/i18n'
import { commonMessages } from '#ui/utils/common-messages'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	createHeader: { id: 'content.mod-groups.create-header', defaultMessage: 'Нова група модів' },
	renameHeader: { id: 'content.mod-groups.rename-header', defaultMessage: 'Перейменувати групу' },
	placeholder: { id: 'content.mod-groups.placeholder', defaultMessage: 'Напр. Оптимізація' },
	hint: {
		id: 'content.mod-groups.hint',
		defaultMessage: 'Група — це підпапка mods/. На час гри моди автоматично переносяться в корінь.',
	},
	create: { id: 'content.mod-groups.create', defaultMessage: 'Створити' },
})

const modal = ref<InstanceType<typeof NewModal>>()
const input = ref<InstanceType<typeof Input>>()
const mode = ref<'create' | 'rename'>('create')
const name = ref('')
const original = ref('')
const busy = ref(false)
let onSubmit: ((name: string) => Promise<void>) | null = null

const header = computed(() =>
	formatMessage(mode.value === 'rename' ? messages.renameHeader : messages.createHeader),
)

function show(kind: 'create' | 'rename', current: string, submit: (name: string) => Promise<void>) {
	mode.value = kind
	name.value = current
	original.value = current
	onSubmit = submit
	busy.value = false
	modal.value?.show()
	void nextTick(() => (input.value?.$el as HTMLElement | undefined)?.querySelector('input')?.focus())
}

async function submit() {
	const value = name.value.trim()
	if (!value || busy.value || !onSubmit) return
	if (mode.value === 'rename' && value === original.value) {
		modal.value?.hide()
		return
	}
	busy.value = true
	try {
		await onSubmit(value)
		modal.value?.hide()
	} finally {
		busy.value = false
	}
}

defineExpose({ show })
</script>
