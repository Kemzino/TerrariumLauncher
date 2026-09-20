import 'dayjs/locale/uk'

import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'

import type { LightboxImage } from '@/components/ui/TerrariumLightbox.vue'

dayjs.extend(relativeTime)

/**
 * Стрічка новин із Discord. Дані готує GitHub Actions у репо TerrariumNews
 * (бот читає канали → news.json); лаунчер лише читає готовий JSON.
 */
export interface NewsAttachment {
	url: string
	name: string
	content_type: string | null
	width: number | null
	height: number | null
}
export interface NewsEmbed {
	title: string | null
	description: string | null
	url: string | null
	image: string | null
}
export interface NewsMessage {
	id: string
	author: string
	avatar: string
	content: string
	timestamp: string
	edited_timestamp?: string | null
	attachments: NewsAttachment[]
	embeds: NewsEmbed[]
	url: string
}
/** Пост форумного каналу: назва + перше повідомлення як обкладинка + відповіді. */
export interface NewsPost {
	id: string
	title: string
	tags: string[]
	message_count: number
	created_at: string | null
	url: string
	starter: NewsMessage | null
	replies?: NewsMessage[]
}
export interface NewsChannel {
	id: string
	name: string
	url: string
	kind?: 'text' | 'forum'
	messages: NewsMessage[]
	posts?: NewsPost[]
}
export interface NewsFeed {
	generated_at: string
	channels: NewsChannel[]
}

export function relativeTimeUk(iso: string) {
	return dayjs(iso).locale('uk').fromNow()
}

export function exactTime(iso: string) {
	return dayjs(iso).format('DD.MM.YYYY HH:mm')
}

/**
 * Мінімальна Discord-розмітка → HTML: жирний, курсив, закреслення, код,
 * посилання; згадки <@id>/<#id>/<@&id> — у нейтральний текст. Спершу
 * екрануємо HTML, щоб вміст повідомлення не міг вставити теги.
 */
export function renderDiscordMarkdown(text: string) {
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

export function isImageAttachment(a: NewsAttachment) {
	return !!a.content_type?.startsWith('image/')
}

/** Усі фото повідомлення (вкладення + картинки ембедів) — гортаються в лайтбоксі разом. */
export function imagesOfMessage(item: NewsMessage, caption?: string): LightboxImage[] {
	const text =
		caption ?? (item.content ? `${item.author}: ${item.content.slice(0, 200)}` : item.author)
	return [
		...item.attachments
			.filter(isImageAttachment)
			.map((a) => ({ url: a.url, name: a.name, caption: text })),
		...item.embeds
			.filter((e) => e.image)
			.map((e) => ({ url: e.image!, name: e.title ?? undefined, caption: text })),
	]
}

/** Усі фото поста: обкладинка + відповіді — одним списком для лайтбокса. */
export function imagesOfPost(post: NewsPost): LightboxImage[] {
	const starter = post.starter ? imagesOfMessage(post.starter, post.title) : []
	const replies = (post.replies ?? []).flatMap((m) => imagesOfMessage(m))
	return [...starter, ...replies]
}
