<script setup lang="ts">
import { ChevronDownIcon, DiscordIcon, ExternalIcon, RefreshCwIcon } from '@modrinth/assets'
import { Button, defineMessages, useVIntl } from '@modrinth/ui'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import NewsMessageItem from '@/components/ui/terrarium-news/NewsMessageItem.vue'
import NewsPostModal from '@/components/ui/terrarium-news/NewsPostModal.vue'
import TerrariumLightbox from '@/components/ui/TerrariumLightbox.vue'
import { openDiscordLink, openTerrariumLink, TERRARIUM_NEWS_URL } from '@/helpers/terrarium-links'
import {
	channelEmoji,
	channelLabel,
	imagesOfMessage,
	imagesOfPost,
	type NewsChannel,
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

// Непрочитане: Discord-id зростають із часом, тож зберігаємо найбільший
// побачений id на канал. Перший запуск (нічого не збережено) — усе «прочитано»,
// щоб не сипати крапками на старі пости.
const READ_KEY = 'terrarium-news-read'
const readIds = ref<Record<string, string>>(readStoredIds())
function readStoredIds(): Record<string, string> {
	try {
		return JSON.parse(localStorage.getItem(READ_KEY) ?? '{}')
	} catch {
		return {}
	}
}
function idNewer(a: string, b: string) {
	return a.length === b.length ? a > b : a.length > b.length
}
function channelItemIds(channel: NewsChannel): string[] {
	return channel.kind === 'forum'
		? (channel.posts ?? []).map((p) => p.id)
		: (channel.messages ?? []).map((m) => m.id)
}
function isUnreadId(channelId: string, id: string) {
	const read = readIds.value[channelId]
	return !!read && idNewer(id, read)
}
function channelHasUnread(channel: NewsChannel) {
	return channelItemIds(channel).some((id) => isUnreadId(channel.id, id))
}
/** Позначити канал прочитаним (найновіший id) */
function markChannelRead(channel: NewsChannel | null) {
	if (!channel) return
	const ids = channelItemIds(channel)
	if (ids.length === 0) return
	const newest = ids.reduce((a, b) => (idNewer(b, a) ? b : a))
	if (readIds.value[channel.id] === newest) return
	readIds.value = { ...readIds.value, [channel.id]: newest }
	localStorage.setItem(READ_KEY, JSON.stringify(readIds.value))
}
/** Після першого завантаження — усі канали без запису вважаємо прочитаними */
function seedReadIds() {
	if (!feed.value) return
	let changed = false
	const next = { ...readIds.value }
	for (const channel of feed.value.channels) {
		if (next[channel.id]) continue
		const ids = channelItemIds(channel)
		if (ids.length === 0) continue
		next[channel.id] = ids.reduce((a, b) => (idNewer(b, a) ? b : a))
		changed = true
	}
	if (changed) {
		readIds.value = next
		localStorage.setItem(READ_KEY, JSON.stringify(next))
	}
}
// Активний розгорнутий канал гравець бачить — після короткої паузи позначаємо
// прочитаним (щоб смужки встигли показатись)
let readTimer: ReturnType<typeof setTimeout> | null = null
watch(
	[activeChannel, collapsed, feed],
	() => {
		if (readTimer) clearTimeout(readTimer)
		if (collapsed.value || !activeChannel.value) return
		const channel = activeChannel.value
		readTimer = setTimeout(() => markChannelRead(channel), 4000)
	},
	{ immediate: true },
)

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
		seedReadIds()
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
			seedReadIds()
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
	<section class="terrarium-widget terrarium-news">
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

		<div v-show="!collapsed" class="terrarium-widget__body terrarium-news__body">
			<div v-if="channels.length" class="terrarium-news__tabs" role="tablist">
				<!-- Вкладка з емодзі в назві: лише емодзі; назва спливає поверх при
				     наведенні, тож рядок вкладок не росте й не переноситься. Без
				     емодзі — повна назва. Назва відкритого каналу — підписом нижче -->
				<button
					v-for="channel in channels"
					:key="channel.id"
					type="button"
					role="tab"
					class="terrarium-news__tab"
					:class="{
						'is-active': channel.id === activeChannel?.id,
						'is-compact': !!channelEmoji(channel.name),
					}"
					:aria-selected="channel.id === activeChannel?.id"
					:aria-label="channelLabel(channel.name)"
					@click="selectChannel(channel.id)"
				>
					<span v-if="channelEmoji(channel.name)" class="terrarium-news__tab-emoji">{{
						channelEmoji(channel.name)
					}}</span>
					<span v-else class="terrarium-news__tab-emoji">#</span>
					<span class="terrarium-news__tab-label">{{ channelLabel(channel.name) }}</span>
					<span v-if="channelHasUnread(channel)" class="terrarium-news__dot" aria-hidden="true" />
				</button>
			</div>

			<div v-if="activeChannel" class="terrarium-news__current">
				<span class="terrarium-news__current-emoji">{{
					channelEmoji(activeChannel.name) ?? '#'
				}}</span>
				<span class="terrarium-news__current-label">{{ channelLabel(activeChannel.name) }}</span>
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
						:class="{ 'is-unread': isUnreadId(activeChannel.id, post.id) }"
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
					:unread="isUnreadId(activeChannel.id, item.id)"
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
	flex-wrap: nowrap;
	gap: 0.3rem;
	min-width: 0;
	margin-bottom: 0.4rem;
}

.terrarium-news__current {
	display: flex;
	align-items: center;
	gap: 0.35rem;
	min-width: 0;
	margin-bottom: 0.5rem;
	font-size: 0.8rem;
	font-weight: 600;
	color: var(--color-contrast);
}

.terrarium-news__current-emoji {
	flex-shrink: 0;
}

.terrarium-news__current-label {
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.terrarium-news__dot {
	display: inline-block;
	width: 0.45rem;
	height: 0.45rem;
	margin-left: 0.3rem;
	border-radius: 999px;
	background: var(--color-orange);
	vertical-align: middle;
}

.terrarium-news__post.is-unread {
	box-shadow: inset 3px 0 0 var(--color-orange);
}

.terrarium-news__tab {
	display: inline-flex;
	align-items: center;
	height: 1.75rem;
	padding: 0 0.6rem;
	border: 1px solid color-mix(in srgb, var(--color-contrast) 12%, transparent);
	border-radius: 999px;
	background: transparent;
	color: var(--color-secondary);
	font: inherit;
	font-size: 0.8rem;
	font-weight: 600;
	line-height: 1;
	white-space: nowrap;
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

.terrarium-news__tab-emoji {
	font-size: 0.95rem;
}

.terrarium-news__tab-label {
	margin-left: 0.3rem;
}

// Згорнута вкладка: лише емодзі; назва спливає поверх сусідів при наведенні /
// фокусі (position: absolute — ширина рядка не змінюється)
.terrarium-news__tab.is-compact {
	position: relative;
	padding: 0 0.45rem;

	.terrarium-news__tab-label {
		position: absolute;
		top: 50%;
		left: calc(100% + 0.25rem);
		z-index: 2;
		margin-left: 0;
		padding: 0.2rem 0.55rem;
		border: 1px solid color-mix(in srgb, var(--color-contrast) 12%, transparent);
		border-radius: 999px;
		background: var(--color-raised-bg);
		box-shadow: 0 4px 14px rgba(0, 0, 0, 0.25);
		color: var(--color-contrast);
		opacity: 0;
		pointer-events: none;
		transform: translate(-0.25rem, -50%);
		transition:
			opacity 0.12s ease,
			transform 0.12s ease;
	}

	&:hover,
	&:focus-visible {
		.terrarium-news__tab-label {
			opacity: 1;
			transform: translate(0, -50%);
		}
	}
}

// Висота віджета задається зовні (колонка hero тягнеться по вікну):
// тіло й список — гнучкі, скрол лише всередині списку
.terrarium-news {
	min-height: 0;
}

.terrarium-news__body {
	display: flex;
	flex: 1 1 auto;
	min-height: 0;
	flex-direction: column;
}

.terrarium-news__list {
	display: flex;
	flex: 1 1 auto;
	flex-direction: column;
	gap: 0.75rem;
	min-width: 0;
	min-height: 6rem;
	overflow-x: hidden;
	max-height: 22rem;
	overflow-y: auto;
	padding-right: 0.25rem;
}

// Поза вузьким режимом верхня межа — доступна висота колонки, не 22rem
@media (min-width: 961px) {
	.terrarium-news__list {
		max-height: none;
	}
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
