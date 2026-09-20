<script setup lang="ts">
import { ChevronDownIcon, DiscordIcon, ExternalIcon, RefreshCwIcon } from '@modrinth/assets'
import { Button, defineMessages, useVIntl } from '@modrinth/ui'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

import NewsMessageItem from '@/components/ui/terrarium-news/NewsMessageItem.vue'
import NewsPostModal from '@/components/ui/terrarium-news/NewsPostModal.vue'
import TerrariumLightbox from '@/components/ui/TerrariumLightbox.vue'
import { openDiscordLink, openTerrariumLink, TERRARIUM_NEWS_URL } from '@/helpers/terrarium-links'
import {
	imagesOfMessage,
	imagesOfPost,
	type NewsFeed,
	type NewsMessage,
	type NewsPost,
	relativeTimeUk,
} from '@/helpers/terrarium-news'

const REFRESH_MS = 5 * 60_000
const CHANNEL_KEY = 'terrarium:news-channel'
const CACHE_KEY = 'terrarium:news-cache'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	title: { id: 'terrarium.news.title', defaultMessage: 'Новини' },
	empty: {
		id: 'terrarium.news.empty',
		defaultMessage: 'Стрічка новин із Discord підключиться найближчим часом.',
	},
	emptyChannel: { id: 'terrarium.news.empty-channel', defaultMessage: 'У каналі поки порожньо' },
	openDiscord: { id: 'terrarium.news.open-discord', defaultMessage: 'Відкрити Discord' },
	openChannel: { id: 'terrarium.news.open-channel', defaultMessage: 'Відкрити канал у Discord' },
	collapse: { id: 'terrarium.news.collapse', defaultMessage: 'Згорнути' },
	expand: { id: 'terrarium.news.expand', defaultMessage: 'Розгорнути' },
	refresh: { id: 'terrarium.news.refresh', defaultMessage: 'Оновити' },
	replies: {
		id: 'terrarium.news.replies',
		defaultMessage:
			'{count, plural, =0 {без відповідей} one {# відповідь} few {# відповіді} other {# відповідей}}',
	},
	openPost: { id: 'terrarium.news.open-post', defaultMessage: 'Відкрити пост у Discord' },
	morePhotos: { id: 'terrarium.news.more-photos', defaultMessage: '+{count}' },
})

const feed = ref<NewsFeed | null>(null)
const loading = ref(false)
const collapsed = ref(false)
const activeChannelId = ref<string | null>(localStorage.getItem(CHANNEL_KEY))
let timer: ReturnType<typeof setInterval> | null = null

const lightbox = ref<InstanceType<typeof TerrariumLightbox> | null>(null)
const postModal = ref<InstanceType<typeof NewsPostModal> | null>(null)

const channels = computed(() => feed.value?.channels ?? [])
const activeChannel = computed(
	() => channels.value.find((c) => c.id === activeChannelId.value) ?? channels.value[0] ?? null,
)

function selectChannel(id: string) {
	activeChannelId.value = id
	localStorage.setItem(CHANNEL_KEY, id)
}

async function refresh() {
	if (loading.value) return
	loading.value = true
	try {
		// raw.githubusercontent кешує до 5 хв; параметр збиває проміжні кеші
		const res = await fetch(`${TERRARIUM_NEWS_URL}?t=${Date.now()}`, { cache: 'no-store' })
		if (!res.ok) throw new Error(`HTTP ${res.status}`)
		const data = (await res.json()) as NewsFeed
		feed.value = data
		localStorage.setItem(CACHE_KEY, JSON.stringify(data))
	} catch (err) {
		// Офлайн або стрічки ще немає — лишаємо останню збережену
		console.warn('Terrarium: не вдалося оновити новини', err)
	} finally {
		loading.value = false
	}
}

function openMessageImage(item: NewsMessage, url: string) {
	const list = imagesOfMessage(item)
	lightbox.value?.show(
		list,
		Math.max(
			0,
			list.findIndex((i) => i.url === url),
		),
	)
}

function coverUrl(post: NewsPost) {
	return post.starter ? imagesOfMessage(post.starter)[0]?.url : undefined
}

function photoCount(post: NewsPost) {
	return imagesOfPost(post).length
}

onMounted(() => {
	const cached = localStorage.getItem(CACHE_KEY)
	if (cached) {
		try {
			feed.value = JSON.parse(cached) as NewsFeed
		} catch {
			localStorage.removeItem(CACHE_KEY)
		}
	}
	void refresh()
	timer = setInterval(() => void refresh(), REFRESH_MS)
})
onBeforeUnmount(() => {
	if (timer) clearInterval(timer)
})
</script>

<template>
	<section class="terrarium-widget">
		<header class="terrarium-widget__head">
			<h3 class="terrarium-widget__title"><DiscordIcon /> {{ formatMessage(messages.title) }}</h3>
			<span class="flex items-center gap-1">
				<Button
					v-tooltip="formatMessage(messages.refresh)"
					type="transparent"
					size="sm"
					icon-only
					:disabled="loading"
					:aria-label="formatMessage(messages.refresh)"
					@click="refresh"
				>
					<RefreshCwIcon :class="{ 'animate-spin': loading }" />
				</Button>
				<Button
					v-tooltip="collapsed ? formatMessage(messages.expand) : formatMessage(messages.collapse)"
					type="transparent"
					size="sm"
					icon-only
					:aria-label="
						collapsed ? formatMessage(messages.expand) : formatMessage(messages.collapse)
					"
					@click="collapsed = !collapsed"
				>
					<ChevronDownIcon class="transition-transform" :class="{ '-rotate-90': collapsed }" />
				</Button>
			</span>
		</header>

		<div v-show="!collapsed" class="terrarium-widget__body">
			<div v-if="channels.length" class="terrarium-news__tabs" role="tablist">
				<button
					v-for="channel in channels"
					:key="channel.id"
					type="button"
					role="tab"
					class="terrarium-news__tab"
					:class="{ 'is-active': channel.id === activeChannel?.id }"
					:aria-selected="channel.id === activeChannel?.id"
					@click="selectChannel(channel.id)"
				>
					#{{ channel.name }}
				</button>
			</div>

			<!-- Форум: список постів; клік — пост цілком у модалці -->
			<div v-if="activeChannel?.kind === 'forum'" class="terrarium-news__list">
				<p v-if="!activeChannel.posts?.length" class="m-0 text-sm text-secondary">
					{{ formatMessage(messages.emptyChannel) }}
				</p>
				<div v-else class="terrarium-news__posts">
					<article
						v-for="post in activeChannel.posts"
						:key="post.id"
						class="terrarium-news__post"
						:title="post.title"
						@click="postModal?.show(post)"
					>
						<div class="terrarium-news__post-cover">
							<img v-if="coverUrl(post)" :src="coverUrl(post)" :alt="post.title" loading="lazy" />
							<span v-if="photoCount(post) > 1" class="terrarium-news__post-more">
								{{ formatMessage(messages.morePhotos, { count: photoCount(post) - 1 }) }}
							</span>
						</div>
						<div class="terrarium-news__post-body">
							<strong class="terrarium-news__post-title">{{ post.title }}</strong>
							<span class="terrarium-news__post-meta">
								<template v-if="post.starter">{{ post.starter.author }} · </template>
								<template v-if="post.created_at">{{ relativeTimeUk(post.created_at) }} · </template>
								{{
									formatMessage(messages.replies, { count: Math.max(0, post.message_count - 1) })
								}}
							</span>
						</div>
						<button
							v-tooltip="formatMessage(messages.openPost)"
							type="button"
							class="terrarium-news__open"
							:aria-label="formatMessage(messages.openPost)"
							@click.stop="openDiscordLink(post.url)"
						>
							<ExternalIcon />
						</button>
					</article>
				</div>
				<Button
					size="sm"
					type="transparent"
					class="self-start"
					@click="openDiscordLink(activeChannel.url)"
				>
					<ExternalIcon /> {{ formatMessage(messages.openChannel) }}
				</Button>
			</div>

			<!-- Текстовий канал: стрічка повідомлень -->
			<div v-else-if="activeChannel" class="terrarium-news__list">
				<p v-if="!activeChannel.messages.length" class="m-0 text-sm text-secondary">
					{{ formatMessage(messages.emptyChannel) }}
				</p>
				<NewsMessageItem
					v-for="item in activeChannel.messages"
					:key="item.id"
					:item="item"
					@open-image="openMessageImage(item, $event)"
				/>
				<Button
					size="sm"
					type="transparent"
					class="self-start"
					@click="openDiscordLink(activeChannel.url)"
				>
					<ExternalIcon /> {{ formatMessage(messages.openChannel) }}
				</Button>
			</div>

			<div v-else class="terrarium-widget__empty">
				<p class="m-0">{{ formatMessage(messages.empty) }}</p>
				<Button size="sm" type="outlined" @click="openTerrariumLink('discord')">
					<ExternalIcon /> {{ formatMessage(messages.openDiscord) }}
				</Button>
			</div>
		</div>
		<TerrariumLightbox ref="lightbox" />
		<NewsPostModal ref="postModal" />
	</section>
</template>

<style scoped lang="scss">
@use './terrarium-widget' as *;

.terrarium-news__tabs {
	display: flex;
	flex-wrap: wrap;
	gap: 0.3rem;
	margin-bottom: 0.6rem;
}

.terrarium-news__tab {
	padding: 0.2rem 0.6rem;
	border: 1px solid color-mix(in srgb, var(--color-contrast) 12%, transparent);
	border-radius: 999px;
	background: transparent;
	color: var(--color-secondary);
	font: inherit;
	font-size: 0.8rem;
	font-weight: 600;
	cursor: pointer;

	&:hover {
		color: var(--color-contrast);
	}

	&.is-active {
		border-color: var(--color-brand);
		background: var(--color-brand-highlight);
		color: var(--color-brand);
	}
}

.terrarium-news__list {
	display: flex;
	flex-direction: column;
	gap: 0.75rem;
	max-height: 22rem;
	overflow-y: auto;
	padding-right: 0.25rem;
}

.terrarium-news__open {
	display: inline-flex;
	padding: 0;
	border: 0;
	background: none;
	color: var(--color-secondary);
	cursor: pointer;

	svg {
		width: 0.9rem;
		height: 0.9rem;
	}

	&:hover {
		color: var(--color-contrast);
	}
}

// Форум: список постів — обкладинка + назва + автор/час/відповіді
.terrarium-news__posts {
	display: flex;
	flex-direction: column;
	gap: 0.4rem;
}

.terrarium-news__post {
	display: flex;
	align-items: center;
	gap: 0.6rem;
	padding: 0.3rem;
	border-radius: var(--radius-md);
	cursor: pointer;
	transition: background-color 0.12s ease;

	&:hover {
		background: var(--color-button-bg);
	}
}

.terrarium-news__post-cover {
	position: relative;
	flex-shrink: 0;
	width: 4.25rem;
	height: 3.25rem;
	overflow: hidden;
	border-radius: var(--radius-sm);
	background: var(--color-button-bg);

	img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
}

.terrarium-news__post-more {
	position: absolute;
	right: 0.2rem;
	bottom: 0.2rem;
	padding: 0 0.3rem;
	border-radius: 999px;
	background: rgba(0, 0, 0, 0.65);
	color: #fff;
	font-size: 0.7rem;
	font-weight: 600;
}

.terrarium-news__post-body {
	display: flex;
	flex: 1;
	min-width: 0;
	flex-direction: column;
	gap: 0.1rem;
}

.terrarium-news__post-title {
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
	color: var(--color-contrast);
}

.terrarium-news__post-meta {
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
	font-size: 0.75rem;
	color: var(--color-secondary);
}
</style>
