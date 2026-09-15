# Реліз лаунчера

Оновлювач — вбудований `tauri-plugin-updater` (форк Modrinth). Лаунчер раз на 5 хв запитує
`https://github.com/Kemzino/TerrariumLauncher/releases/latest/download/latest.json`, порівнює
`version` зі своєю (semver) і, якщо новіша, качає інсталятор, перевіряє minisign-підпис
і ставить оновлення при виході з лаунчера (NSIS, `passive` режим).

## Що де

| Файл                                      | Навіщо                                                                          |
| ----------------------------------------- | ------------------------------------------------------------------------------- |
| `apps/app/tauri-release.conf.json`        | фіча `updater`, публічний ключ, ендпоінт                                        |
| `apps/app/tauri.local-release.conf.json`  | вимикає `beforeBuildCommand` (вкладений turbo падає на Windows)                 |
| `.github/workflows/terrarium-release.yml` | **основний реліз**: бамп версії → тег → збірка → підпис → `latest.json` → реліз |
| `scripts/terrarium-release.ps1`           | те саме локально (запасний шлях)                                                |
| `~/.tauri/terrarium-launcher.key`         | **приватний ключ підпису — не в репо, зроби бекап**                             |

Без приватного ключа підписати оновлення неможливо → усі встановлені лаунчери
перестануть оновлюватись. Публічний ключ вшитий у `tauri-release.conf.json`;
якщо міняти ключ, стара база користувачів оновлення не прийме.

## Як випустити версію (GitHub Actions — основний шлях)

Воркфлоу `.github/workflows/terrarium-release.yml`.

1. **Один раз**: додати секрет репо `TAURI_SIGNING_PRIVATE_KEY` — вміст файлу
   `~/.tauri/terrarium-launcher.key` (Settings → Secrets and variables → Actions), або:
   ```powershell
   gh secret set TAURI_SIGNING_PRIVATE_KEY -R Kemzino/TerrariumLauncher < "$env:USERPROFILE\.tauri\terrarium-launcher.key"
   ```
   Якщо ключ із паролем — ще `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
2. GitHub → **Actions → Terrarium Launcher release → Run workflow**: вибрати гілку
   (`terrarium`), ввести версію `X.Y.Z`. Воркфлоу сам:
   - підніме версію в `apps/app-frontend/package.json`, `apps/app/Cargo.toml`,
     `packages/app-lib/Cargo.toml`, `Cargo.lock`, закомітить `chore: release vX.Y.Z` і поставить тег;
   - збере інсталятор на Windows-раннері з фічею `updater`, підпише `.sig`;
   - згенерує release notes з комітів від попереднього тега і `latest.json`;
   - створить реліз з трьома асетами.
     Збірка з нуля ~20–30 хв, з кешем cargo швидше.
3. Альтернатива: підняти версію у файлах руками, закомітити, `git tag vX.Y.Z && git push origin terrarium vX.Y.Z` —
   пуш тега запускає той самий воркфлоу (лише етап збірки).

Після публікації release notes можна відредагувати на GitHub — оновлювач читає `latest.json`, а не текст релізу.

## Ручний реліз (запасний шлях)

Якщо Actions недоступні — той самий результат локально:

1. Підняти `version` в `apps/app-frontend/package.json` (звідти її бере `tauri.conf.json`),
   `apps/app/Cargo.toml` і `packages/app-lib/Cargo.toml`. Оновлювач порівнює semver — без бампа нову версію ніхто не побачить.
2. Закомітити і запушити (`git push origin HEAD`) — тег ставиться на HEAD, GitHub вимагає, щоб коміт був на remote.
3. ```powershell
   $env:TAURI_SIGNING_PRIVATE_KEY = "$env:USERPROFILE\.tauri\terrarium-launcher.key"
   .\scripts\terrarium-release.ps1 -NotesFile .\notes.md        # або -Notes "текст"
   ```
   Прапорець `-Draft` створює чернетку (оновлювач її не бачить, поки не опублікуєш у GitHub),
   `-SkipBuild` — лише публікація вже зібраного інсталятора.
4. Скрипт створює тег `vX.Y.Z` на поточному коміті і реліз з трьома асетами:
   `Terrarium-Launcher_X.Y.Z_x64-setup.exe`, `.exe.sig`, `latest.json`.

## Перевірка

Поставити попередню версію, запустити — за кілька секунд має з'явитись попап "Update available".
Лог: `%APPDATA%\TerrariumLauncher\launcher_logs\`.

## Обмеження

- Лише Windows x64. Для macOS/Linux треба додати відповідні платформи в `latest.json`
  і зібрати їх на відповідних машинах (див. `.github/workflows/theseus-release.yml` як приклад).
- Інсталятор не має code-signing сертифіката → SmartScreen попереджає при першому запуску.
  Оновлення це не блокує (updater перевіряє minisign, не Authenticode).
