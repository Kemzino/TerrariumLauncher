<template>
	<span
		v-tooltip="formatMessage(messages.tooltip)"
		class="inline-flex items-center gap-1 rounded-full bg-highlight-orange px-2 py-0.5 align-middle text-xs font-semibold text-orange"
	>
		<CurseForgeIcon class="size-3.5 shrink-0" aria-hidden="true" />
		CurseForge
	</span>
</template>

<script setup lang="ts">
// Terrarium: значок, що проєкт із CurseForge — єдина видима різниця з Modrinth
import { CurseForgeIcon } from '@modrinth/assets'

import { defineMessages, useVIntl } from '../../composables/i18n'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	tooltip: {
		id: 'project.curseforge-badge.tooltip',
		defaultMessage: 'Цей проєкт з CurseForge',
	},
})
</script>
