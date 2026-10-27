// Terrarium: черга «докачай вручну» — файли CurseForge, які автор заборонив качати
// стороннім лаунчерам і яких нема на Modrinth. Вікно (TerrariumCurseForgeBlockedModal)
// змонтоване в App.vue і відкривається, щойно сюди щось потрапляє.
import { ref } from 'vue'

import type { CurseForgeBlockedFile } from '@/helpers/curseforge'

export interface CurseForgeBlockedRequest {
	instanceId: string
	/** Установка збірки ще йде — файли кладемо після неї */
	jobId?: string | null
	files: CurseForgeBlockedFile[]
}

export const curseforgeBlockedRequest = ref<CurseForgeBlockedRequest | null>(null)

export function showCurseForgeBlocked(request: CurseForgeBlockedRequest) {
	if (request.files.length === 0) return
	curseforgeBlockedRequest.value = request
}
