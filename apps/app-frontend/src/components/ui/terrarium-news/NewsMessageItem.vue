<script setup lang="ts">
import { ExternalIcon } from '@modrinth/assets'
import { Avatar, defineMessages, useVIntl } from '@modrinth/ui'

import { openDiscordLink } from '@/helpers/terrarium-links'
import {
	exactTime,
	isImageAttachment,
	type NewsMessage,
	relativeTimeUk,
	renderDiscordMarkdown,
} from '@/helpers/terrarium-news'

/** Одне повідомлення Discord: аватар, автор, час, текст, ембеди, вкладення. */
withDefaults(
	defineProps<{
		item: NewsMessage
		/** Без аватара/автора — коли контекст і так зрозумілий (обкладинка поста) */
		compact?: boolean
		/** Гравець ще не бачив це повідомлення */
		unread?: boolean
	}>(),
	{ compact: false, unread: false },
)
const emit = defineEmits<{ openImage: [url: string] }>()

const { formatMessage } = useVIntl()
const messages = defineMessages({
	openMessage: { id: 'terrarium.news.open-message', defaultMessage: 'Відкрити в Discord' },
	attachment: { id: 'terrarium.news.attachment', defaultMessage: 'Вкладення: {name}' },
})

// Посилання в тексті — назовні (Discord-застосунок або браузер), не всередині webview
function onContentClick(event: MouseEvent) {
	const link = (event.target as HTMLElement).closest('a')
	if (!link?.href) return
	event.preventDefault()
	void openDiscordLink(link.href)
}
</script>

<template>
	<article class="news-message" :class="{ 'is-compact': compact, 'is-unread': unread }">
		<Avatar v-if="!compact" :src="item.avatar" size="1.75rem" class="news-message__avatar" circle />
		<div class="news-message__main">
			<div v-if="!compact" class="news-message__meta">
				<strong>{{ item.author }}</strong>
				<span v-tooltip="exactTime(item.timestamp)">{{ relativeTimeUk(item.timestamp) }}</span>
				<button
					v-tooltip="formatMessage(messages.openMessage)"
					type="button"
					class="news-message__open"
					:aria-label="formatMessage(messages.openMessage)"
					@click="openDiscordLink(item.url)"
				>
					<ExternalIcon />
				</button>
			</div>
			<!-- eslint-disable-next-line vue/no-v-html -- вміст екрановано в renderDiscordMarkdown -->
			<div
				v-if="item.content"
				class="news-message__content"
				@click="onContentClick"
				v-html="renderDiscordMarkdown(item.content)"
			/>
			<div
				v-for="embed in item.embeds"
				:key="embed.url ?? embed.title ?? ''"
				class="news-message__embed"
			>
				<strong v-if="embed.title">{{ embed.title }}</strong>
				<span v-if="embed.description">{{ embed.description }}</span>
				<img
					v-if="embed.image"
					:src="embed.image"
					alt=""
					loading="lazy"
					class="news-message__image"
					@click="emit('openImage', embed.image)"
				/>
			</div>
			<div
				v-if="item.attachments.length"
				class="news-message__attachments"
				:class="{ 'is-grid': item.attachments.filter(isImageAttachment).length > 1 }"
			>
				<template v-for="a in item.attachments" :key="a.url">
					<img
						v-if="isImageAttachment(a)"
						:src="a.url"
						:alt="a.name"
						loading="lazy"
						class="news-message__image"
						@click="emit('openImage', a.url)"
					/>
					<button v-else type="button" class="news-message__file" @click="openDiscordLink(a.url)">
						<ExternalIcon /> {{ formatMessage(messages.attachment, { name: a.name }) }}
					</button>
				</template>
			</div>
		</div>
	</article>
</template>

<style scoped lang="scss">
.news-message {
	display: flex;
	gap: 0.6rem;
	min-width: 0;
}

/* Непрочитане: смужка зліва */
.news-message.is-unread {
	position: relative;
	padding-left: 0.6rem;
	margin-left: -0.6rem;
	border-left: 3px solid var(--color-orange);
	border-radius: 4px;
}

.news-message__avatar {
	flex-shrink: 0;
}

.news-message__main {
	display: flex;
	flex: 1;
	min-width: 0;
	flex-direction: column;
	gap: 0.25rem;
}

.news-message__meta {
	display: flex;
	align-items: center;
	gap: 0.4rem;
	font-size: 0.78rem;
	color: var(--color-secondary);

	strong {
		color: var(--color-contrast);
	}
}

.news-message__open {
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

.news-message__content {
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

.news-message__embed {
	display: flex;
	flex-direction: column;
	gap: 0.2rem;
	padding: 0.4rem 0.6rem;
	border-left: 3px solid var(--color-brand);
	border-radius: var(--radius-sm);
	background: var(--color-button-bg);
	font-size: 0.85rem;
}

.news-message__attachments {
	display: flex;
	flex-wrap: wrap;
	gap: 0.4rem;

	// Кілька фото — квадратні мініатюри у два стовпці, одне — на всю ширину
	&.is-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;

		.news-message__image {
			width: 100%;
			aspect-ratio: 1;
			max-height: none;
			object-fit: cover;
		}
	}
}

.news-message__image {
	max-width: 100%;
	max-height: 14rem;
	border-radius: var(--radius-md);
	cursor: zoom-in;
	transition: filter 0.12s ease;

	&:hover {
		filter: brightness(1.1);
	}
}

.news-message__file {
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
