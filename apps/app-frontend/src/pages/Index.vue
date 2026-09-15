<script setup lang="ts">
import { PlayIcon } from '@modrinth/assets'
import { defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { useQuery } from '@tanstack/vue-query'
import { computed } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'

import LibrarySection from '@/components/ui/library/index.vue'
import { libraryScrollTop } from '@/components/ui/library/view-state'
import TerrariumHero from '@/components/ui/TerrariumHero.vue'
import { instanceListQueryOptions } from '@/pages/instance/query-options'
import { useRootBreadcrumb } from '@/providers/breadcrumbs'

defineOptions({
	name: 'HomePage',
})

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()

const messages = defineMessages({
	home: {
		id: 'app.navigation.home',
		defaultMessage: 'Home',
	},
})

useRootBreadcrumb({
	slot: 'root',
	id: 'home',
	label: formatMessage(messages.home),
	to: '/',
	visual: { type: 'icon', component: PlayIcon },
})

onBeforeRouteLeave(() => {
	libraryScrollTop.value = document.querySelector('.app-viewport')?.scrollTop ?? 0
})

const instancesQuery = useQuery(instanceListQueryOptions())
instancesQuery.suspense().catch(handleError)
const instances = computed(() => instancesQuery.data.value ?? [])
</script>

<template>
	<div class="terrarium-home">
		<TerrariumHero />
		<section class="terrarium-home__library p-6">
			<LibrarySection :instances="instances" />
		</section>
	</div>
</template>

<style scoped lang="scss">
.terrarium-home {
	display: flex;
	flex-direction: column;
	min-height: 100%;
}

// Hero займає весь перший екран; бібліотека — далі по скролу.
.terrarium-home > :deep(.terrarium-hero) {
	min-height: calc(100vh - var(--top-bar-height, 3rem));
	flex-shrink: 0;
}

.terrarium-home__library {
	background: var(--color-bg);
}
</style>
