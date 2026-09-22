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
/** Згадки в тексті — щоб показати їх словами й у кольорі ролі, а не як id. */
export interface NewsMentions {
	users?: Record<string, string>
	roles?: Record<string, { name: string; color: string | null }>
	channels?: Record<string, string>
}
export interface NewsMessage {
	id: string
	author: string
	/** Колір ніка — найвища роль автора з кольором (як на сервері Discord) */
	author_color?: string | null
	avatar: string
	content: string
	mentions?: NewsMentions
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

function escapeHtml(text: string) {
	return text
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;')
}

/** Колір згадки: лише безпечний hex, інакше нейтральний. */
function mentionStyle(color: string | null | undefined) {
	return color && /^#[0-9a-f]{6}$/i.test(color)
		? ` style="color:${color};background:${color}1f"`
		: ''
}

/**
 * Discord-розмітка → HTML: жирний, курсив, закреслення, код, цитати, заголовки,
 * посилання, кастомні емодзі (картинкою з CDN) і згадки <@id>/<#id>/<@&id> —
 * словами й у кольорі ролі, якщо стрічка дала довідник. Спершу екрануємо HTML,
 * щоб вміст повідомлення не міг вставити теги.
 */
export function renderDiscordMarkdown(text: string, mentions?: NewsMentions) {
	const escaped = escapeHtml(text)
	const mention = (label: string, color?: string | null) =>
		`<span class="mention"${mentionStyle(color)}>${escapeHtml(label)}</span>`
	return (
		escaped
			.replace(/&lt;@&amp;(\d+)&gt;/g, (_, id: string) => {
				const role = mentions?.roles?.[id]
				return role ? mention(`@${role.name}`, role.color) : mention('@роль')
			})
			.replace(/&lt;@!?(\d+)&gt;/g, (_, id: string) => {
				const name = mentions?.users?.[id]
				return mention(name ? `@${name}` : '@учасник')
			})
			.replace(/&lt;#(\d+)&gt;/g, (_, id: string) => {
				const name = mentions?.channels?.[id]
				return mention(name ? `#${name}` : '#канал')
			})
			// Кастомні емодзі <:name:id> / <a:name:id> — з CDN Discord
			.replace(
				/&lt;(a?):(\w+):(\d+)&gt;/g,
				(_, animated: string, name: string, id: string) =>
					`<img class="emoji" src="https://cdn.discordapp.com/emojis/${id}.${animated ? 'gif' : 'webp'}?size=32" alt=":${name}:" title=":${name}:" loading="lazy">`,
			)
			.replace(/```([\s\S]*?)```/g, '<pre>$1</pre>')
			.replace(/`([^`]+)`/g, '<code>$1</code>')
			.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
			.replace(/__(.+?)__/g, '<u>$1</u>')
			.replace(/~~(.+?)~~/g, '<s>$1</s>')
			.replace(/(^|[^*])\*([^*\n]+)\*/g, '$1<em>$2</em>')
			.replace(/^-# (.*)$/gm, '<small>$1</small>')
			.replace(/^# (.*)$/gm, '<strong class="text-lg">$1</strong>')
			.replace(/^## (.*)$/gm, '<strong>$1</strong>')
			.replace(/^### (.*)$/gm, '<strong>$1</strong>')
			.replace(/\[([^\]]+)\]\((https?:\/\/[^)\s]+)\)/g, '<a href="$2" target="_blank">$1</a>')
			.replace(/(^|[^"'>])(https?:\/\/[^\s<]+)/g, '$1<a href="$2" target="_blank">$2</a>')
			// Цитати: сусідні рядки «> …» — одним блоком
			.replace(/^&gt;(?: (.*))?$/gm, '<blockquote>$1</blockquote>')
			.replace(/<\/blockquote>\n<blockquote>/g, '<br>')
			.replace(/\n/g, '<br>')
	)
}

/**
 * Емодзі на початку назви каналу (🔆новини-create → 🔆) — для згорнутих
 * вкладок. Немає — null, тоді вкладка завжди з повною назвою.
 */
export function channelEmoji(name: string): string | null {
	const first = [...new Intl.Segmenter(undefined, { granularity: 'grapheme' }).segment(name)][0]
		?.segment
	return first && /\p{Extended_Pictographic}/u.test(first) ? first : null
}

/** Назва каналу без емодзі на початку. */
export function channelLabel(name: string) {
	const emoji = channelEmoji(name)
	return emoji ? name.slice(emoji.length).replace(/^[\s・|·-]+/, '') : name
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
