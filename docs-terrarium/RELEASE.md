# Реліз лаунчера

Оновлювач — вбудований `tauri-plugin-updater` (форк Modrinth). Лаунчер раз на 5 хв запитує
`https://github.com/Kemzino/TerrariumLauncher/releases/latest/download/latest.json`, порівнює
`version` зі своєю (semver) і, якщо новіша, качає інсталятор, перевіряє minisign-підпис
і ставить оновлення при виході з лаунчера (NSIS, `passive` режим).

## Що де

| Файл | Навіщо |
| --- | --- |
| `apps/app/tauri-release.conf.json` | фіча `updater`, публічний ключ, ендпоінт |
| `apps/app/tauri.local-release.conf.json` | вимикає `beforeBuildCommand` (вкладений turbo падає на Windows) |
| `scripts/terrarium-release.ps1` | збірка + підпис + `latest.json` + `gh release create` |
| `~/.tauri/terrarium-launcher.key` | **приватний ключ підпису — не в репо, зроби бекап** |

Без приватного ключа підписати оновлення неможливо → усі встановлені лаунчери
перестануть оновлюватись. Публічний ключ вшитий у `tauri-release.conf.json`;
якщо міняти ключ, стара база користувачів оновлення не прийме.

## Як випустити версію

1. Підняти `version` в `apps/app-frontend/package.json` (звідти її бере `tauri.conf.json`)
   і в `apps/app/Cargo.toml`. Оновлювач порівнює semver — без бампа нову версію ніхто не побачить.
2. Закомітити.
3. ```powershell
   $env:TAURI_SIGNING_PRIVATE_KEY = "$env:USERPROFILE\.tauri\terrarium-launcher.key"
   .\scripts\terrarium-release.ps1 -NotesFile .\notes.md        # або -Notes "текст"
   ```
   Прапорець `-Draft` створює чернетку (оновлювач її не бачить, поки не опублікуєш у GitHub),
   `-SkipBuild` — лише публікація вже зібраного інсталятора.
4. Скрипт створює тег `vX.Y.Z` на поточному коміті і реліз з трьома асетами:
   `Terrarium-Launcher_X.Y.Z_x64-setup.exe`, `.exe.sig`, `latest.json`.
5. Запушити гілку: `git push origin terrarium`.

## Перевірка

Поставити попередню версію, запустити — за кілька секунд має з'явитись попап "Update available".
Лог: `%APPDATA%\TerrariumLauncher\launcher_logs\`.

## Обмеження

- Лише Windows x64. Для macOS/Linux треба додати відповідні платформи в `latest.json`
  і зібрати їх на відповідних машинах (див. `.github/workflows/theseus-release.yml` як приклад).
- Інсталятор не має code-signing сертифіката → SmartScreen попереджає при першому запуску.
  Оновлення це не блокує (updater перевіряє minisign, не Authenticode).