<script setup lang="ts">
import 'dayjs/locale/uk'

import { ChevronDownIcon, DiscordIcon, ExternalIcon, RefreshCwIcon } from '@modrinth/assets'
import { Avatar, Button, defineMessages, useVIntl } from '@modrinth/ui'
import { openUrl } from '@tauri-apps/plugin-opener'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

import TerrariumLightbox, { type LightboxImage } from '@/components/ui/TerrariumLightbox.vue'
import { openTerrariumLink, TERRARIUM_NEWS_URL } from '@/helpers/terrarium-links'

dayjs.extend(relativeTime)

/**
 * Стрічка новин із Discord. Дані готує GitHub Actions у репо TerrariumNews
 * (бот читає канали → news.json); лаунчер лише читає готовий JSON.
 */
interface NewsAttachment {
	url: string
	name: string
	content_type: string | null
	width: number | null
	height: number | null
}
interface NewsEmbed {
	title: string | null
	description: string | null
	url: string | null
	image: string | null
}
interface NewsMessage {
	id: string
	author: string
	avatar: string
	content: string
	timestamp: string
	attachments: NewsAttachment[]
	embeds: NewsEmbed[]
	url: string
}
/** Пост форумного каналу: назва + перше повідомлення як обкладинка. */
interface NewsPost {
	id: string
	title: string
	tags: string[]
	message_count: number
	created_at: string | null
	url: string
	starter: NewsMessage | null
}
interface NewsChannel {
	id: string
	name: string
	url: string
	kind?: 'text' | 'forum'
	messages: NewsMessage[]
	posts?: NewsPost[]
}
interface NewsFeed {
	generated_at: string
	channels: NewsChannel[]
}

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
	openMessage: { id: 'terrarium.news.open-message', defaultMessage: 'Відкрити в Discord' },
	collapse: { id: 'terrarium.news.collapse', defaultMessage: 'Згорнути' },
	expand: { id: 'terrarium.news.expand', defaultMessage: 'Розгорнути' },
	refresh: { id: 'terrarium.news.refresh', defaultMessage: 'Оновити' },
	attachment: { id: 'terrarium.news.attachment', defaultMessage: 'Вкладення: {name}' },
	replies: {
		id: 'terrarium.news.replies',
		defaultMessage:
			'{count, plural, =0 {без відповідей} one {# відповідь} few {# відповіді} other {# відповідей}}',
	},
	openPost: { id: 'terrarium.news.open-post', defaultMessage: 'Відкрити пост у Discord' },
	morePhotos: { id: 'terrarium.news.more-photos', defaultMessage: '+{count}' },
	edited: { id: 'terrarium.news.edited', defaultMessage: '(змінено)' },
})

const feed = ref<NewsFeed | null>(null)
const loading = ref(false)
const collapsed = ref(false)
const activeChannelId = ref<string | null>(localStorage.getItem(CHANNEL_KEY))
let timer: ReturnType<typeof setInterval> | null = null

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

function relative(iso: string) {
	return dayjs(iso).locale('uk').fromNow()
}

/**
 * Мінімальна Discord-розмітка → HTML: жирний, курсив, закреслення, код,
 * посилання; згадки <@id>/<#id>/<@&id> — у нейтральний текст. Спершу
 * екрануємо HTML, щоб вміст повідомлення не міг вставити теги.
 */
function renderContent(text: string) {
	const escaped = text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
	return escaped
		.replace(/&lt;@&amp;\d+&gt;/g, '@роль')
		.replace(/&lt;@!?\d+&gt;/g, '@учасник')
		.replace(/&lt;#\d+&gt;/g, '#канал')
		.replace(/&lt;a?:(\w+):\d+&gt;/g, ':$1:')
		.replace(/```([\s\S]*?)```/g, '<pre>$1</pre>')
		.replace(/`([^`]+)`/g, '<code>$1</code>')
		.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
		.replace(/__(.+?)__/g, '<u>$1</u>')
		.replace(/~~(.+?)~~/g, '<s>$1</s>')
		.replace(/(^|[^*])\*([^*\n]+)\*/g, '$1<em>$2</em>')
		.replace(/^&gt; (.*)$/gm, '<blockquote>$1</blockquote>')
		.replace(/^# (.*)$/gm, '<strong class="text-lg">$1</strong>')
		.replace(/^## (.*)$/gm, '<strong>$1</strong>')
		.replace(/^### (.*)$/gm, '<strong>$1</strong>')
		.replace(/\[([^\]]+)\]\((https?:\/\/[^)\s]+)\)/g, '<a href="$2" target="_blank">$1</a>')
		.replace(/(^|[^"'>])(https?:\/\/[^\s<]+)/g, '$1<a href="$2" target="_blank">$2</a>')
		.replace(/\n/g, '<br>')
}

function isImage(a: NewsAttachment) {
	return !!a.content_type?.startsWith('image/')
}

const lightbox = ref<InstanceType<typeof TerrariumLightbox> | null>(null)

/** Усі фото повідомлення (вкладення + картинки ембедів) — гортаються в лайтбоксі разом. */
function imagesOf(item: NewsMessage): LightboxImage[] {
	const caption = item.content ? `${item.author}: ${item.content.slice(0, 200)}` : item.author
	return [
		...item.attachments.filter(isImage).map((a) => ({ url: a.url, name: a.name, caption })),
		...item.embeds
			.filter((e) => e.image)
			.map((e) => ({ url: e.image!, name: e.title ?? undefined, caption })),
	]
}

function coverOf(post: NewsPost): LightboxImage[] {
	if (!post.starter) return []
	return imagesOf(post.starter).map((img) => ({ ...img, caption: post.title }))
}

function openPostImage(post: NewsPost, index = 0) {
	const list = coverOf(post)
	if (list.length) lightbox.value?.show(list, index)
	else void openUrl(post.url)
}

function openImage(item: NewsMessage, url: string) {
	const list = imagesOf(item)
	lightbox.value?.show(
		list,
		Math.max(
			0,
			list.findIndex((i) => i.url === url),
		),
	)
}

// Посилання в тексті відкриваємо в системному браузері, а не всередині вебв'ю
function onContentClick(event: MouseEvent) {
	const link = (event.target as HTMLElement).closest('a')
	if (!link?.href) return
	event.preventDefault()
	void openUrl(link.href)
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
						@click="openPostImage(post)"
					>
						<div class="terrarium-news__post-cover">
							<img
								v-if="coverOf(post)[0]"
								:src="coverOf(post)[0].url"
								:alt="post.title"
								loading="lazy"
							/>
							<span v-if="coverOf(post).length > 1" class="terrarium-news__post-more">
								{{ formatMessage(messages.morePhotos, { count: coverOf(post).length - 1 }) }}
							</span>
						</div>
						<div class="terrarium-news__post-body">
							<strong class="terrarium-news__post-title">{{ post.title }}</strong>
							<span class="terrarium-news__post-meta">
								<template v-if="post.starter">{{ post.starter.author }} · </template>
								<template v-if="post.created_at">{{ relative(post.created_at) }} · </template>
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
							@click.stop="openUrl(post.url)"
						>
							<ExternalIcon />
						</button>
					</article>
				</div>
				<Button size="sm" type="transparent" class="self-start" @click="openUrl(activeChannel.url)">
					<ExternalIcon /> {{ formatMessage(messages.openChannel) }}
				</Button>
			</div>

			<div v-else-if="activeChannel" class="terrarium-news__list">
				<p v-if="!activeChannel.messages.length" class="m-0 text-sm text-secondary">
					{{ formatMessage(messages.emptyChannel) }}
				</p>
				<article v-for="item in activeChannel.messages" :key="item.id" class="terrarium-news__item">
					<Avatar :src="item.avatar" size="1.75rem" class="terrarium-news__avatar" circle />
					<div class="terrarium-news__main">
						<div class="terrarium-news__meta">
							<strong>{{ item.author }}</strong>
							<span v-tooltip="dayjs(item.timestamp).format('DD.MM.YYYY HH:mm')">
								{{ relative(item.timestamp) }}
							</span>
							<button
								v-tooltip="formatMessage(messages.openMessage)"
								type="button"
								class="terrarium-news__open"
								:aria-label="formatMessage(messages.openMessage)"
								@click="openUrl(item.url)"
							>
								<ExternalIcon />
							</button>
						</div>
						<!-- eslint-disable-next-line vue/no-v-html -- вміст екрановано в renderContent -->
						<div
							v-if="item.content"
							class="terrarium-news__content"
							@click="onContentClick"
							v-html="renderContent(item.content)"
						/>
						<div
							v-for="embed in item.embeds"
							:key="embed.url ?? embed.title ?? ''"
							class="terrarium-news__embed"
						>
							<strong v-if="embed.title">{{ embed.title }}</strong>
							<span v-if="embed.description">{{ embed.description }}</span>
							<img
								v-if="embed.image"
								:src="embed.image"
								alt=""
								loading="lazy"
								class="terrarium-news__embed-image"
								@click="openImage(item, embed.image)"
							/>
						</div>
						<div
							v-if="item.attachments.length"
							class="terrarium-news__attachments"
							:class="{ 'is-grid': item.attachments.filter(isImage).length > 1 }"
						>
							<template v-for="a in item.attachments" :key="a.url">
								<img
									v-if="isImage(a)"
									:src="a.url"
									:alt="a.name"
									loading="lazy"
									class="terrarium-news__image"
									@click="openImage(item, a.url)"
								/>
								<button v-else type="button" class="terrarium-news__file" @click="openUrl(a.url)">
									<ExternalIcon /> {{ formatMessage(messages.attachment, { name: a.name }) }}
								</button>
							</template>
						</div>
					</div>
				</article>
				<Button size="sm" type="transparent" class="self-start" @click="openUrl(activeChannel.url)">
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

.terrarium-news__item {
	display: flex;
	gap: 0.6rem;
	min-width: 0;
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

.terrarium-news__avatar {
	flex-shrink: 0;
}

.terrarium-news__main {
	display: flex;
	flex: 1;
	min-width: 0;
	flex-direction: column;
	gap: 0.25rem;
}

.terrarium-news__meta {
	display: flex;
	align-items: center;
	gap: 0.4rem;
	font-size: 0.78rem;
	color: var(--color-secondary);

	strong {
		color: var(--color-contrast);
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

.terrarium-news__content {
	font-size: 0.9rem;
	line-height: 1.4;
	color: var(--color-base);
	overflow-wrap: anywhere;

	:deep(a) {
		color: var(--color-brand);
	}

	:deep(code),
	:deep(pre) {
		padding: 0.05rem 0.3rem;
		border-radius: var(--radius-sm);
		background: var(--color-button-bg);
		font-size: 0.85em;
	}

	:deep(pre) {
		white-space: pre-wrap;
		padding: 0.4rem 0.6rem;
	}

	:deep(blockquote) {
		margin: 0.2rem 0;
		padding-left: 0.6rem;
		border-left: 3px solid var(--color-divider);
		color: var(--color-secondary);
	}
}

.terrarium-news__embed {
	display: flex;
	flex-direction: column;
	gap: 0.2rem;
	padding: 0.4rem 0.6rem;
	border-left: 3px solid var(--color-brand);
	border-radius: var(--radius-sm);
	background: var(--color-button-bg);
	font-size: 0.85rem;

	img {
		max-width: 100%;
		border-radius: var(--radius-sm);
	}
}

.terrarium-news__attachments {
	display: flex;
	flex-wrap: wrap;
	gap: 0.4rem;

	// Кілька фото — квадратні мініатюри у два стовпці, одне — на всю ширину
	&.is-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;

		.terrarium-news__image {
			width: 100%;
			aspect-ratio: 1;
			max-height: none;
			object-fit: cover;
		}
	}
}

.terrarium-news__image,
.terrarium-news__embed-image {
	max-width: 100%;
	max-height: 12rem;
	border-radius: var(--radius-md);
	cursor: zoom-in;
	transition: filter 0.12s ease;

	&:hover {
		filter: brightness(1.1);
	}
}

.terrarium-news__file {
	display: inline-flex;
	align-items: center;
	gap: 0.3rem;
	padding: 0.2rem 0.5rem;
	border: 1px solid var(--color-divider);
	border-radius: var(--radius-sm);
	background: transparent;
	color: var(--color-base);
	font: inherit;
	font-size: 0.8rem;
	cursor: pointer;

	svg {
		width: 0.9rem;
		height: 0.9rem;
	}
}
</style>
