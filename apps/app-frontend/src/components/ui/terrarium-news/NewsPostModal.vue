<script setup lang="ts">
import { ExternalIcon } from '@modrinth/assets'
import { Button, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { computed, ref } from 'vue'

import TerrariumLightbox from '@/components/ui/TerrariumLightbox.vue'
import { openDiscordLink } from '@/helpers/terrarium-links'
import { imagesOfPost, type NewsPost, relativeTimeUk } from '@/helpers/terrarium-news'

import NewsMessageItem from './NewsMessageItem.vue'

/** Пост форуму цілком: обкладинка з текстом і фото, далі відповіді. */
const { formatMessage } = useVIntl()

const messages = defineMessages({
	replies: {
		id: 'terrarium.news.replies',
		defaultMessage:
			'{count, plural, =0 {без відповідей} one {# відповідь} few {# відповіді} other {# відповідей}}',
	},
	repliesHeading: { id: 'terrarium.news.replies-heading', defaultMessage: 'Відповіді' },
	moreReplies: {
		id: 'terrarium.news.more-replies',
		defaultMessage:
			'Ще {count, plural, one {# відповідь} few {# відповіді} other {# відповідей}} — у Discord',
	},
	openPost: { id: 'terrarium.news.open-post', defaultMessage: 'Відкрити пост у Discord' },
})

const modal = ref<InstanceType<typeof NewModal>>()
const lightbox = ref<InstanceType<typeof TerrariumLightbox> | null>(null)
const post = ref<NewsPost | null>(null)

const images = computed(() => (post.value ? imagesOfPost(post.value) : []))
const replies = computed(() => post.value?.replies ?? [])
// message_count рахує і стартове повідомлення; у JSON — лише останні N відповідей
const hiddenReplies = computed(() =>
	post.value ? Math.max(0, post.value.message_count - 1 - replies.value.length) : 0,
)

function openImage(url: string) {
	const index = images.value.findIndex((i) => i.url === url)
	lightbox.value?.show(images.value, Math.max(0, index))
}

function show(next: NewsPost) {
	post.value = next
	modal.value?.show()
}

defineExpose({ show })
</script>

<template>
	<NewModal
		ref="modal"
		:header="post?.title ?? ''"
		scrollable
		width="42rem"
		max-width="calc(100vw - 2rem)"
	>
		<div v-if="post" class="news-post">
			<p class="news-post__meta">
				<template v-if="post.starter">{{ post.starter.author }} · </template>
				<template v-if="post.created_at">{{ relativeTimeUk(post.created_at) }} · </template>
				{{ formatMessage(messages.replies, { count: Math.max(0, post.message_count - 1) }) }}
				<span v-for="tag in post.tags" :key="tag" class="news-post__tag">{{ tag }}</span>
			</p>

			<NewsMessageItem v-if="post.starter" :item="post.starter" compact @open-image="openImage" />

			<template v-if="replies.length || hiddenReplies">
				<h4 class="news-post__replies-heading">{{ formatMessage(messages.repliesHeading) }}</h4>
				<div class="news-post__replies">
					<NewsMessageItem
						v-for="reply in replies"
						:key="reply.id"
						:item="reply"
						@open-image="openImage"
					/>
					<Button
						v-if="hiddenReplies"
						type="transparent"
						size="sm"
						class="self-start"
						@click="openDiscordLink(post.url)"
					>
						<ExternalIcon /> {{ formatMessage(messages.moreReplies, { count: hiddenReplies }) }}
					</Button>
				</div>
			</template>
		</div>
		<template #actions>
			<div class="flex items-center justify-end">
				<Button v-if="post" type="outlined" @click="openDiscordLink(post.url)">
					<ExternalIcon /> {{ formatMessage(messages.openPost) }}
				</Button>
			</div>
		</template>
		<TerrariumLightbox ref="lightbox" />
	</NewModal>
</template>

<style scoped lang="scss">
.news-post {
	display: flex;
	flex-direction: column;
	gap: 0.75rem;
}

.news-post__meta {
	display: flex;
	flex-wrap: wrap;
	align-items: center;
	gap: 0.35rem;
	margin: 0;
	font-size: 0.85rem;
	color: var(--color-secondary);
}

.news-post__tag {
	padding: 0.05rem 0.5rem;
	border-radius: 999px;
	background: var(--color-button-bg);
	font-size: 0.75rem;
	color: var(--color-contrast);
}

.news-post__replies-heading {
	margin: 0.5rem 0 0;
	padding-top: 0.75rem;
	border-top: 1px solid var(--color-divider);
	font-size: 0.85rem;
	font-weight: 700;
	letter-spacing: 0.04em;
	text-transform: uppercase;
	color: var(--color-secondary);
}

.news-post__replies {
	display: flex;
	flex-direction: column;
	gap: 0.85rem;
}
</style>
