# План: Linux і macOS

Стан на 2026-09-16: лаунчер збирається й оновлюється лише для Windows x64. Код
Terrarium (групи модів, синхронізація, канали, оновлювач) платформо-незалежний;
конфіг Tauri (`apps/app/tauri.conf.json`) уже описує bundle для Linux (deb,
AppImage, rpm) і macOS (dmg) — це спадок Modrinth, який збирає всі три платформи.

## 0. Рішення, які треба ухвалити до старту

| Питання           | Варіанти                                                                | Рекомендація                                                                                                                                                                                                                                                     |
| ----------------- | ----------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Формат Linux      | AppImage (єдиний, який уміє самооновлюватись) / .deb / .rpm             | **AppImage** як основний + .deb для зручності; .rpm — за запитом                                                                                                                                                                                                 |
| Архітектури Linux | x86_64 / aarch64                                                        | лише **x86_64** (гравці на ARM-Linux — одиниці)                                                                                                                                                                                                                  |
| macOS             | universal (Intel+Apple Silicon в одному .app) / окремо aarch64 і x86_64 | **universal** — один файл, один запис у `latest.json`, так робить Modrinth                                                                                                                                                                                       |
| Підпис macOS      | Apple Developer Program ($99/рік) → підпис + нотаризація / без підпису  | **Без сертифіката** на старті: гравець один раз знімає карантин (`xattr -cr`) або відкриває через ПКМ → Open. Купувати, коли буде ≥ кілька macOS-гравців. Без нотаризації **оновлювач на macOS працює**, але після кожного оновлення Gatekeeper знову попереджає |
| Мінімальна macOS  | `minimumSystemVersion` у конфігу                                        | залишити як у Modrinth                                                                                                                                                                                                                                           |
| Хто тестує        | реальний Mac і Linux потрібні для smoke-тесту                           | Linux — VM (Ubuntu 22.04/24.04) тут же; macOS — знайти одного гравця з Mac або орендувати (MacStadium/Scaleway годинно)                                                                                                                                          |

## 1. Збірка в CI (основна робота)

`.github/workflows/terrarium-release.yml` → матриця з трьох джоб + фінальна джоба публікації.

1. **`build` матриця**
   - `windows-latest` → NSIS (як зараз).
   - `ubuntu-22.04` (не `latest`: старіший glibc → бінарник запускається на більшій кількості дистрибутивів) →
     `apt-get install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf`,
     `tauri build --bundles appimage,deb`.
   - `macos-latest` (Apple Silicon) → `rustup target add x86_64-apple-darwin aarch64-apple-darwin`,
     `tauri build --target universal-apple-darwin --bundles app,dmg`, `TAURI_BUNDLER_DMG_IGNORE_CI=true`.
   - Усі три: `TAURI_SIGNING_PRIVATE_KEY` (той самий minisign-ключ), `createUpdaterArtifacts: true`
     → крім інсталятора з'являються updater-артефакти: `.AppImage.sig`, `.app.tar.gz` + `.app.tar.gz.sig`.
   - Кожна джоба → `actions/upload-artifact` з перейменованими асетами (без пробілів):
     `Terrarium-Launcher_X_x64-setup.exe(.sig)`, `Terrarium-Launcher_X_amd64.AppImage(.sig)`,
     `Terrarium-Launcher_X_amd64.deb`, `Terrarium-Launcher_X_universal.dmg`, `Terrarium-Launcher.app.tar.gz(.sig)`.
2. **`release` джоба** (`needs: build`, `ubuntu-latest`): завантажує артефакти, генерує `latest.json` на всі платформи
   і робить `gh release create` з усіма файлами. Джоба `tag` (бамп версії) не змінюється.
3. Кеш cargo окремо на кожну ОС (`cache-key: terrarium-release-${{ runner.os }}`).
4. Час: Linux ~15 хв, macOS universal ~35–45 хв (дві архітектури), Windows ~20 хв — паралельно.

## 2. Маніфест оновлювача

`latest.json` за форматом Tauri, ключі платформ:

```json
"platforms": {
  "windows-x86_64": { "signature": "...", "url": ".../Terrarium-Launcher_X_x64-setup.exe" },
  "linux-x86_64":   { "signature": "...", "url": ".../Terrarium-Launcher_X_amd64.AppImage" },
  "darwin-aarch64": { "signature": "...", "url": ".../Terrarium-Launcher.app.tar.gz" },
  "darwin-x86_64":  { "signature": "...", "url": ".../Terrarium-Launcher.app.tar.gz" }
}
```

- macOS: обидва ключі вказують на один universal `.app.tar.gz`.
- Linux: оновлювач працює **лише для AppImage**; хто поставив .deb — бачить сповіщення «є нова версія»
  (`checkLinuxUpdates` в `App.vue` уже вміє, коли оновлення вимкнені) і качає руками.
- Windows-частина не змінюється — встановлені 1.0.x оновлюються далі.

## 3. Код і конфіг

- `apps/app/tauri-release.conf.json`: додати `"macOS": { "signingIdentity": "-" }` (ad-hoc підпис — без нього
  Apple Silicon узагалі не запускає бінарник), лишити `minimumSystemVersion` з базового конфігу.
- Перевірити `bundle.linux.deb.depends` (зараз порожньо) — Modrinth покладається на автодетект tauri; ок.
- Тексти: release notes і підказка про SmartScreen → додати абзац про Gatekeeper (macOS) і `chmod +x` для AppImage.
- `docs-terrarium/RELEASE.md`: оновити список асетів і кроки перевірки на кожній ОС.
- Локальний `scripts/terrarium-release.ps1` лишається Windows-only запасним шляхом; для інших ОС — лише CI.
- Java: лаунчер сам качає JRE під платформу (логіка Modrinth) — нічого не робити, але перевірити на Linux/macOS.

## 4. Тестування (smoke на кожній ОС)

1. Встановити з асету релізу → лаунчер стартує, головна вантажиться, фон/логотип на місці.
2. Встановити збірку (клієнт) → гра запускається (Java підтягнулась, NeoForge поставився).
3. Групи модів: створити, перетягнути мод, «Уміст збірки» по групах.
4. Оновлення лаунчера: поставити попередню версію → має підтягнути нову (Linux — AppImage; macOS — після
   оновлення перевірити, що .app запускається, не «damaged»).
5. Адмін-функції (ключ, тест-канал, синхронізація) — хоча б один прогін на Linux.

## 5. Порядок робіт і оцінка

| Крок | Що                                                                        | Оцінка                  |
| ---- | ------------------------------------------------------------------------- | ----------------------- |
| 1    | Матриця CI + артефакти + `release`-джоба + `latest.json` на всі платформи | 3–4 год + прогони CI    |
| 2    | Конфіг macOS (ad-hoc підпис), тексти, документація                        | 1 год                   |
| 3    | Smoke на Linux (VM)                                                       | 1–2 год                 |
| 4    | Smoke на macOS (потрібен Mac)                                             | 1–2 год + доступ до Mac |
| 5    | Реліз `1.1.0` з трьома платформами                                        | прогін CI ~45 хв        |

Ризики: (а) macOS без сертифіката — частина гравців злякається попередження; (б) AppImage на деяких
дистрибутивах потребує `libfuse2`; (в) universal-збірка на CI довга — якщо заважатиме, перейти на дві окремі
джоби macOS (aarch64 + x86_64) з двома записами в `latest.json`.
