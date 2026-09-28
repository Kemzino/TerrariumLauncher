import type { ButtonColor, ButtonInteraction, ButtonSize, ButtonType } from './types'

/**
 * Класи кнопки одним рядком — те саме, що складає `ButtonFrame`.
 *
 * Потрібні там, де компонент-кнопка задорога: у довгих списках кожен
 * `IconButton` — це ще два компоненти Vue на елемент, і на сітці з сотень
 * плиток вони помітно сповільнюють появу. Такі місця малюють звичайний
 * `<button>` із цими класами, тож вигляд лишається той самий.
 */

const baseClasses = [
	// Base
	'relative inline-flex min-w-0 shrink-0 items-center justify-center',
	'whitespace-nowrap border-0 no-underline',
	// Interactions
	'touch-manipulation cursor-pointer select-none transition-[background-color,color,box-shadow,filter,opacity,transform] duration-150 ease-out',
	'enabled:active:scale-[0.97]',
	// Hovering
	'[&:not(:disabled):not([aria-disabled=true]):hover]:brightness-[--hover-brightness]',
	// Accessibility
	'[&:not(:disabled):not([aria-disabled=true]):focus-visible]:brightness-[--hover-brightness] focus-visible:outline-none [&:not(:disabled):not([aria-disabled=true]):focus-visible]:ring-4 [&:not(:disabled):not([aria-disabled=true]):focus-visible]:ring-brand-shadow',
	'disabled:cursor-not-allowed disabled:opacity-50',
	'[&[aria-disabled=true]]:cursor-not-allowed [&[aria-disabled=true]]:opacity-50',
].join(' ')

const sizeClasses: Record<ButtonSize, string> = {
	xs: 'h-7 gap-1 rounded-lg px-1.5 text-sm font-semibold leading-5 [&>svg]:size-4 [&>svg]:min-h-4 [&>svg]:min-w-4 [&>svg]:shrink-0',
	sm: 'h-8 gap-1 rounded-[10px] px-1.5 text-sm font-semibold leading-5 [&>svg]:size-4 [&>svg]:min-h-4 [&>svg]:min-w-4 [&>svg]:shrink-0',
	md: 'h-9 gap-1.5 rounded-xl px-2.5 text-base font-semibold leading-5 [&>svg]:size-5 [&>svg]:min-h-5 [&>svg]:min-w-5 [&>svg]:shrink-0',
	lg: 'h-10 gap-2 rounded-[14px] px-4 text-base font-semibold leading-5 [&>svg]:size-5 [&>svg]:min-h-5 [&>svg]:min-w-5 [&>svg]:shrink-0',
	xl: 'h-12 gap-2 rounded-2xl px-3.5 text-base font-extrabold leading-5 [&>svg]:size-6 [&>svg]:min-h-6 [&>svg]:min-w-6 [&>svg]:shrink-0',
}

const iconOnlySizeClasses: Record<ButtonSize, string> = {
	xs: 'w-7 !px-0',
	sm: 'w-8 !px-0',
	md: 'w-9 !px-0',
	lg: 'w-10 !px-0',
	xl: 'w-12 !px-0',
}

const typeClasses: Record<ButtonType, string> = {
	base: 'button-frame--base bg-surface-4 text-contrast [&>svg]:text-primary',
	colored:
		'button-frame--colored bg-[--button-color] text-[var(--color-accent-contrast)] [&>svg]:text-inherit',
	'colored-text':
		'button-frame--colored-text bg-surface-4 text-[--button-color] [&>svg]:text-inherit',
	outlined:
		'button-frame--outlined bg-transparent text-[var(--button-color,var(--color-contrast))] [&>svg]:text-[var(--button-color,var(--color-base))]',
	quiet: 'button-frame--quiet bg-transparent [&>svg]:text-inherit',
}

const interactionClasses: Record<ButtonInteraction, string> = {
	surface:
		'[&:not(:disabled):not([aria-disabled=true]):hover]:bg-surface-4 [&:not(:disabled):not([aria-disabled=true]):focus-visible]:bg-surface-4',
	filled:
		'[&:not(:disabled):not([aria-disabled=true]):hover]:!bg-[--button-color] [&:not(:disabled):not([aria-disabled=true]):focus-visible]:!bg-[--button-color] [&:not(:disabled):not([aria-disabled=true]):hover]:!text-[var(--color-accent-contrast)] [&:not(:disabled):not([aria-disabled=true]):focus-visible]:!text-[var(--color-accent-contrast)]',
	none: '[&:not(:disabled):not([aria-disabled=true]):hover]:!brightness-100 [&:not(:disabled):not([aria-disabled=true]):focus-visible]:!brightness-100',
}

const colorVariables: Record<ButtonColor, string> = {
	brand: 'var(--color-brand)',
	red: 'var(--color-red)',
	orange: 'var(--color-orange)',
	green: 'var(--color-green)',
	blue: 'var(--color-blue)',
	purple: 'var(--color-purple)',
	medal_promotion: 'var(--medal-promotion-text-orange, var(--color-orange))',
}

export interface ButtonClassOptions {
	type?: ButtonType
	size?: ButtonSize
	interaction?: ButtonInteraction
	iconOnly?: boolean
	circular?: boolean
}

export function buttonClasses({
	type = 'base',
	size = 'md',
	interaction = 'surface',
	iconOnly = false,
	circular = false,
}: ButtonClassOptions = {}): string {
	return [
		baseClasses,
		typeClasses[type],
		type === 'quiet' ? interactionClasses[interaction] : '',
		sizeClasses[size],
		iconOnly ? iconOnlySizeClasses[size] : '',
		circular ? '!rounded-full' : '',
	]
		.filter(Boolean)
		.join(' ')
}

/** Колірна змінна кнопки — те саме, що `ButtonFrame` кладе в style */
export function buttonColorStyle(
	type: ButtonType,
	color?: ButtonColor,
): Record<string, string> | undefined {
	if ((type === 'outlined' || type === 'quiet') && !color) return undefined
	if (type !== 'colored' && type !== 'colored-text' && type !== 'outlined' && type !== 'quiet') {
		return undefined
	}

	return { '--button-color': colorVariables[color ?? 'brand'] }
}
