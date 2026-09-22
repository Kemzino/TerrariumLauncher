import { useMagicKeys } from '@vueuse/core'
import { effectScope, type Ref } from 'vue'

let shift: Ref<boolean> | undefined

/**
 * Чи затиснутий Shift — спільний на весь застосунок.
 *
 * `useMagicKeys()` у кожному рядку списку вішав власний набір слухачів на
 * `window`: у списку на сотні модів це сотні слухачів і стільки ж реактивних
 * оновлень на кожне натискання клавіші. Тут слухачі створюються один раз, у
 * власному (від'єднаному) scope — щоб вони не зникли, коли розмонтується той
 * компонент, який звернувся першим.
 */
export function useShiftKey(): Ref<boolean> {
	if (!shift) {
		shift = effectScope(true).run(() => useMagicKeys().shift)!
	}
	return shift
}
