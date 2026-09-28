<script setup lang="ts">
import type { Component, CSSProperties } from 'vue'
import { computed, ref } from 'vue'

import { buttonClasses, buttonColorStyle } from './button-classes'
import type {
	ButtonColor,
	ButtonInteraction,
	ButtonNativeType,
	ButtonSize,
	ButtonType,
} from './types'

const props = withDefaults(
	defineProps<{
		as: string | Component
		type?: ButtonType
		color?: ButtonColor
		size?: ButtonSize
		interaction?: ButtonInteraction
		iconOnly?: boolean
		circular?: boolean
		nativeType?: ButtonNativeType
	}>(),
	{
		type: 'base',
		size: 'md',
		interaction: 'surface',
		iconOnly: false,
		circular: false,
		nativeType: undefined,
	},
)

const element = ref<HTMLElement | null>(null)
const classes = computed(() =>
	buttonClasses({
		type: props.type,
		size: props.size,
		interaction: props.interaction,
		iconOnly: props.iconOnly,
		circular: props.circular,
	}),
)
const style = computed(() => buttonColorStyle(props.type, props.color) as CSSProperties | undefined)

defineExpose({ element })
</script>

<template>
	<component
		:is="as"
		ref="element"
		data-button
		:type="props.nativeType"
		:class="classes"
		:style="style"
	>
		<slot />
	</component>
</template>

<style scoped>
.button-frame--base,
.button-frame--colored-text {
	box-shadow:
		inset 0 0 0 1px var(--surface-5),
		0 1px 1px rgba(0, 0, 0, 0.12);
}

.button-frame--colored {
	box-shadow:
		0 0 0 1px color-mix(in srgb, var(--button-color) 30%, transparent),
		0 2px 4px rgba(0, 0, 0, 0.04),
		0 5px 8px rgba(0, 0, 0, 0.04),
		0 10px 18px rgba(0, 0, 0, 0.03),
		0 24px 48px rgba(0, 0, 0, 0.03);
}

.button-frame--colored::before {
	position: absolute;
	inset: 0;
	padding: 1px;
	pointer-events: none;
	content: '';
	border-radius: inherit;
	background: linear-gradient(180deg, rgba(255, 255, 255, 0.3), rgba(255, 255, 255, 0));
	-webkit-mask:
		linear-gradient(#000 0 0) content-box,
		linear-gradient(#000 0 0);
	-webkit-mask-composite: xor;
	mask-composite: exclude;
}

.button-frame--outlined {
	box-shadow: 0 0 0 1px var(--button-color, var(--surface-5));
}

.button-frame--quiet {
	color: var(--button-color, var(--color-base));
}
</style>
