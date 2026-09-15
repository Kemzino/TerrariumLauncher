<script setup lang="ts">
import {
	BookOpenIcon,
	BoxesIcon,
	CompassIcon,
	DiscordIcon,
	HomeIcon,
	ImageIcon,
	LibraryIcon,
	PlusIcon,
	SettingsIcon,
	ShirtIcon,
	XIcon,
} from '@modrinth/assets'
import { defineMessages, IconButton, useVIntl } from '@modrinth/ui'
import { nextTick, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import terrariumLogo from '@/assets/terrarium/logo.png'
import { openTerrariumLink } from '@/helpers/terrarium-links'

const props = defineProps<{ open: boolean }>()
const emit = defineEmits<{ close: []; openSettings: []; newInstance: [] }>()

const { formatMessage } = useVIntl()
const route = useRoute()
const router = useRouter()

const messages = defineMessages({
	sectionCommunity: { id: 'terrarium.nav.section-community', defaultMessage: 'Спільнота' },
	linkRules: { id: 'terrarium.links.rules', defaultMessage: 'Правила сервера' },
	linkDiscord: { id: 'terrarium.links.discord', defaultMessage: 'Наш Discord' },
	menu: { id: 'terrarium.nav.menu', defaultMessage: 'Меню' },
	close: { id: 'terrarium.nav.close', defaultMessage: 'Закрити меню' },
	home: { id: 'terrarium.nav.home', defaultMessage: 'Головна' },
	library: { id: 'terrarium.nav.library', defaultMessage: 'Бібліотека' },
	browseModpacks: { id: 'terrarium.nav.browse-modpacks', defaultMessage: 'Пошук збірок' },
	browseMods: { id: 'terrarium.nav.browse-mods', defaultMessage: 'Пошук модів' },
	skins: { id: 'terrarium.nav.skins', defaultMessage: 'Скіни' },
	screenshots: { id: 'terrarium.nav.screenshots', defaultMessage: 'Скріншоти' },
	newInstance: { id: 'terrarium.nav.new-instance', defaultMessage: 'Новий примірник' },
	settings: { id: 'terrarium.nav.settings', defaultMessage: 'Налаштування' },
	sectionPlay: { id: 'terrarium.nav.section-play', defaultMessage: 'Гра' },
	sectionDiscover: { id: 'terrarium.nav.section-discover', defaultMessage: 'Знайти' },
	sectionMore: { id: 'terrarium.nav.section-more', defaultMessage: 'Інше' },
})

async function go(path: string) {
	emit('close')
	if (route.path !== path) await router.push(path)
}

async function goLibrary() {
	emit('close')
	if (route.path !== '/') await router.push('/')
	await nextTick()
	// Даємо сторінці відрендеритись, потім плавно скролимо до бібліотеки під hero
	setTimeout(() => {
		document
			.querySelector('.terrarium-home__library')
			?.scrollIntoView({ behavior: 'smooth', block: 'start' })
	}, 120)
}

function isActive(path: string) {
	return route.path === path
}

// Будь-який перехід закриває меню — навіть якщо він стався не звідси
watch(
	() => route.fullPath,
	() => {
		if (props.open) emit('close')
	},
)
</script>

<template>
	<Teleport to="body">
		<Transition name="terrarium-nav">
			<div v-if="open" class="terrarium-nav" @keydown.esc="emit('close')">
				<div class="terrarium-nav__backdrop" @click="emit('close')"></div>
				<nav class="terrarium-nav__panel" :aria-label="formatMessage(messages.menu)">
					<div class="terrarium-nav__head">
						<img :src="terrariumLogo" alt="" class="terrarium-nav__logo" />
						<IconButton type="quiet" :label="formatMessage(messages.close)" @click="emit('close')">
							<XIcon />
						</IconButton>
					</div>

					<p class="terrarium-nav__section">{{ formatMessage(messages.sectionPlay) }}</p>
					<button class="terrarium-nav__item" :class="{ active: isActive('/') }" @click="go('/')">
						<HomeIcon /> {{ formatMessage(messages.home) }}
					</button>
					<button class="terrarium-nav__item" @click="goLibrary">
						<LibraryIcon /> {{ formatMessage(messages.library) }}
					</button>

					<p class="terrarium-nav__section">{{ formatMessage(messages.sectionDiscover) }}</p>
					<button
						class="terrarium-nav__item"
						:class="{ active: isActive('/browse/modpack') }"
						@click="go('/browse/modpack')"
					>
						<CompassIcon /> {{ formatMessage(messages.browseModpacks) }}
					</button>
					<button
						class="terrarium-nav__item"
						:class="{ active: isActive('/browse/mod') }"
						@click="go('/browse/mod')"
					>
						<BoxesIcon /> {{ formatMessage(messages.browseMods) }}
					</button>
					<button
						class="terrarium-nav__item"
						:class="{ active: isActive('/skins') }"
						@click="go('/skins')"
					>
						<ShirtIcon /> {{ formatMessage(messages.skins) }}
					</button>
					<button
						class="terrarium-nav__item"
						:class="{ active: isActive('/screenshots') }"
						@click="go('/screenshots')"
					>
						<ImageIcon /> {{ formatMessage(messages.screenshots) }}
					</button>

					<p class="terrarium-nav__section">{{ formatMessage(messages.sectionMore) }}</p>
					<button class="terrarium-nav__item" @click="emit('newInstance')">
						<PlusIcon /> {{ formatMessage(messages.newInstance) }}
					</button>
					<button class="terrarium-nav__item" @click="emit('openSettings')">
						<SettingsIcon /> {{ formatMessage(messages.settings) }}
					</button>

					<p class="terrarium-nav__section">{{ formatMessage(messages.sectionCommunity) }}</p>
					<div class="terrarium-nav__links">
						<button
							v-tooltip="formatMessage(messages.linkDiscord)"
							class="terrarium-nav__link terrarium-nav__link--discord"
							:aria-label="formatMessage(messages.linkDiscord)"
							@click="openTerrariumLink('discord')"
						>
							<DiscordIcon />
						</button>
						<button
							v-tooltip="formatMessage(messages.linkRules)"
							class="terrarium-nav__link"
							:aria-label="formatMessage(messages.linkRules)"
							@click="openTerrariumLink('rules')"
						>
							<BookOpenIcon />
						</button>
					</div>
				</nav>
			</div>
		</Transition>
	</Teleport>
</template>

<style scoped lang="scss">
.terrarium-nav {
	position: fixed;
	inset: 0;
	z-index: 60;
}

.terrarium-nav__backdrop {
	position: absolute;
	inset: 0;
	background: rgba(0, 0, 0, 0.45);
	backdrop-filter: blur(2px);
}

.terrarium-nav__panel {
	position: absolute;
	top: 0;
	bottom: 0;
	left: 0;
	width: 17rem;
	display: flex;
	flex-direction: column;
	gap: 0.15rem;
	padding: 0.75rem 0.75rem 1rem;
	background: var(--color-raised-bg);
	border-right: 1px solid var(--color-divider);
	box-shadow: 12px 0 40px rgba(0, 0, 0, 0.35);
	overflow-y: auto;
}

.terrarium-nav__head {
	display: flex;
	align-items: center;
	justify-content: space-between;
	padding: 0.25rem 0.25rem 0.75rem;
}

.terrarium-nav__logo {
	height: 2.75rem;
	width: auto;
	-webkit-user-drag: none;
}

.terrarium-nav__section {
	margin: 0.75rem 0 0.25rem;
	padding: 0 0.6rem;
	font-size: 0.7rem;
	font-weight: 700;
	letter-spacing: 0.08em;
	text-transform: uppercase;
	color: var(--color-secondary);
}

.terrarium-nav__links {
	display: flex;
	gap: 0.5rem;
	padding: 0.25rem 0.6rem;
}

.terrarium-nav__link {
	display: inline-flex;
	align-items: center;
	justify-content: center;
	width: 2.4rem;
	height: 2.4rem;
	border: 1px solid color-mix(in srgb, var(--color-contrast) 12%, transparent);
	border-radius: 999px;
	background: color-mix(in srgb, var(--color-contrast) 6%, transparent);
	color: var(--color-base);
	cursor: pointer;
	transition:
		background-color 0.12s ease,
		color 0.12s ease,
		border-color 0.12s ease;
}

.terrarium-nav__link svg {
	width: 1.2rem;
	height: 1.2rem;
}

.terrarium-nav__link:hover,
.terrarium-nav__link:focus-visible {
	background: var(--color-brand);
	border-color: var(--color-brand);
	color: var(--color-accent-contrast);
}

.terrarium-nav__link--discord:hover,
.terrarium-nav__link--discord:focus-visible {
	background: #5865f2;
	border-color: #5865f2;
	color: #fff;
}

.terrarium-nav__item {
	display: flex;
	align-items: center;
	gap: 0.65rem;
	width: 100%;
	padding: 0.6rem 0.6rem;
	border: 0;
	border-radius: var(--radius-md);
	background: transparent;
	color: var(--color-base);
	font: inherit;
	font-weight: 600;
	text-align: left;
	cursor: pointer;
	transition:
		background-color 0.12s ease,
		color 0.12s ease;

	svg {
		width: 1.15rem;
		height: 1.15rem;
		flex-shrink: 0;
	}

	&:hover {
		background: var(--color-button-bg);
		color: var(--color-contrast);
	}

	&.active {
		background: var(--color-brand-highlight);
		color: var(--color-brand);
	}
}

.terrarium-nav-enter-active,
.terrarium-nav-leave-active {
	transition: opacity 0.18s ease;

	.terrarium-nav__panel {
		transition: transform 0.2s ease;
	}
}

.terrarium-nav-enter-from,
.terrarium-nav-leave-to {
	opacity: 0;

	.terrarium-nav__panel {
		transform: translateX(-1.5rem);
	}
}
</style>
