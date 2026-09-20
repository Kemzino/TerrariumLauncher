<script setup lang="ts">
import { ChevronDownIcon, DiscordIcon, ExternalIcon } from '@modrinth/assets'
import { Button, defineMessages, useVIntl } from '@modrinth/ui'
import { ref } from 'vue'

import { openTerrariumLink } from '@/helpers/terrarium-links'

/**
 * Заглушка стрічки новин. Дані з'являться, коли підключимо Discord (канал
 * новин через бота/вебхук); контракт елемента — див. `NewsItem`, щоб бекенд
 * було чим замінити без переверстки.
 */
export interface NewsItem {
	id: string
	title: string
	body: string
	published_at: string
	url?: string
}

const { formatMessage } = useVIntl()

const messages = defineMessages({
	title: { id: 'terrarium.news.title', defaultMessage: 'Новини' },
	empty: {
		id: 'terrarium.news.empty',
		defaultMessage: 'Стрічка новин із Discord підключиться найближчим часом.',
	},
	openDiscord: { id: 'terrarium.news.open-discord', defaultMessage: 'Відкрити Discord' },
	collapse: { id: 'terrarium.news.collapse', defaultMessage: 'Згорнути' },
	expand: { id: 'terrarium.news.expand', defaultMessage: 'Розгорнути' },
})

// Поки джерела немає — порожній список; компонент уже вміє його рендерити
const items = ref<NewsItem[]>([])
const collapsed = ref(false)
</script>

<template>
	<section class="terrarium-widget">
		<header class="terrarium-widget__head">
			<h3 class="terrarium-widget__title"><DiscordIcon /> {{ formatMessage(messages.title) }}</h3>
			<Button
				v-tooltip="collapsed ? formatMessage(messages.expand) : formatMessage(messages.collapse)"
				type="transparent"
				size="sm"
				icon-only
				:aria-label="collapsed ? formatMessage(messages.expand) : formatMessage(messages.collapse)"
				@click="collapsed = !collapsed"
			>
				<ChevronDownIcon class="transition-transform" :class="{ '-rotate-90': collapsed }" />
			</Button>
		</header>
		<div v-show="!collapsed" class="terrarium-widget__body">
			<ul v-if="items.length" class="terrarium-news__list">
				<li v-for="item in items" :key="item.id" class="terrarium-news__item">
					<span class="terrarium-news__date">{{ item.published_at }}</span>
					<strong class="terrarium-news__item-title">{{ item.title }}</strong>
					<p class="terrarium-news__item-body">{{ item.body }}</p>
				</li>
			</ul>
			<div v-else class="terrarium-widget__empty">
				<p class="m-0">{{ formatMessage(messages.empty) }}</p>
				<Button size="sm" type="outlined" @click="openTerrariumLink('discord')">
					<ExternalIcon /> {{ formatMessage(messages.openDiscord) }}
				</Button>
			</div>
		</div>
	</section>
</template>

<style scoped lang="scss">
@use './terrarium-widget' as *;

.terrarium-news__list {
	display: flex;
	flex-direction: column;
	gap: 0.75rem;
	margin: 0;
	padding: 0;
	list-style: none;
}

.terrarium-news__item {
	display: flex;
	flex-direction: column;
	gap: 0.15rem;
}

.terrarium-news__date {
	font-size: 0.75rem;
	color: var(--color-secondary);
}

.terrarium-news__item-title {
	color: var(--color-contrast);
}

.terrarium-news__item-body {
	margin: 0;
	font-size: 0.9rem;
	color: var(--color-base);
}
</style>
