<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

// Фонові фото збірки. Достатньо покласти файли в assets/terrarium/backgrounds —
// Vite підхопить їх при збірці, нічого реєструвати не треба.
const images = Object.values(
	import.meta.glob<string>('../../assets/terrarium/backgrounds/*.{jpg,jpeg,png,webp}', {
		eager: true,
		import: 'default',
	}),
)

const props = withDefaults(defineProps<{ intervalMs?: number }>(), { intervalMs: 9000 })

// Стартуємо з випадкового кадру, щоб при кожному запуску фон був іншим
const current = ref(images.length > 0 ? Math.floor(Math.random() * images.length) : 0)
const hasImages = computed(() => images.length > 0)
const next = computed(() => (current.value + 1) % images.length)

let timer: ReturnType<typeof setInterval> | null = null

onMounted(() => {
	if (images.length > 1) {
		timer = setInterval(() => {
			current.value = next.value
		}, props.intervalMs)
	}
})

onBeforeUnmount(() => {
	if (timer) clearInterval(timer)
})
</script>

<template>
	<div class="terrarium-backdrop" aria-hidden="true">
		<template v-if="hasImages">
			<TransitionGroup name="backdrop-fade">
				<img :key="images[current]" :src="images[current]" class="terrarium-backdrop__image" />
			</TransitionGroup>
		</template>
		<div v-else class="terrarium-backdrop__placeholder"></div>
		<div class="terrarium-backdrop__overlay"></div>
	</div>
</template>

<style scoped lang="scss">
.terrarium-backdrop {
	position: absolute;
	inset: 0;
	overflow: hidden;
	background: var(--color-bg);
}

.terrarium-backdrop__image {
	position: absolute;
	inset: 0;
	width: 100%;
	height: 100%;
	object-fit: cover;
	// Повільний «наїзд» камери, щоб фото не виглядало статичною картинкою
	animation: backdrop-zoom 14s ease-out forwards;
}

.terrarium-backdrop__placeholder {
	position: absolute;
	inset: 0;
	background:
		radial-gradient(80% 70% at 15% 100%, var(--terrarium-yellow-soft) 0%, transparent 60%),
		radial-gradient(70% 60% at 100% 0%, rgba(31, 95, 214, 0.35) 0%, transparent 60%),
		linear-gradient(160deg, var(--surface-2) 0%, var(--surface-1) 100%);
}

.terrarium-backdrop__placeholder::after {
	content: '';
	position: absolute;
	inset: 0;
	background-image: linear-gradient(
		135deg,
		transparent 0 48%,
		rgba(255, 255, 255, 0.025) 48% 52%,
		transparent 52% 100%
	);
	background-size: 180px 180px;
}

.terrarium-backdrop__overlay {
	position: absolute;
	inset: 0;
	background:
		linear-gradient(180deg, rgba(11, 21, 38, 0.15) 0%, rgba(11, 21, 38, 0.55) 60%, rgba(11, 21, 38, 0.85) 100%),
		linear-gradient(90deg, rgba(11, 21, 38, 0.55) 0%, rgba(11, 21, 38, 0) 55%);
}

.light-mode .terrarium-backdrop__overlay {
	background:
		linear-gradient(180deg, rgba(223, 233, 247, 0.1) 0%, rgba(223, 233, 247, 0.55) 60%, rgba(223, 233, 247, 0.9) 100%),
		linear-gradient(90deg, rgba(223, 233, 247, 0.5) 0%, rgba(223, 233, 247, 0) 55%);
}

.backdrop-fade-enter-active,
.backdrop-fade-leave-active {
	transition: opacity 1.6s ease;
}

.backdrop-fade-enter-from,
.backdrop-fade-leave-to {
	opacity: 0;
}

@keyframes backdrop-zoom {
	from {
		transform: scale(1);
	}
	to {
		transform: scale(1.08);
	}
}
</style>
