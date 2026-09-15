# Terrarium Launcher — архітектура

Форк `modrinth/code` (гілка `terrarium`, upstream-remote збережено для майбутніх мерджів).

## Цільові вимоги

1. Гра з ліцензією (Microsoft) **і** в офлайн-режимі.
2. Автооновлення збірки модів з GitHub Releases.
3. Оновлення **не перезаписує конфіги гравця**.
4. Оновлення **не видаляє клієнтські моди гравця**.
5. Базова версія — MC 1.21.1 / NeoForge, але лаунчер має вміти й інші версії/лоадери.

## Що вже є в upstream (не пишемо самі)

| Потреба | Де | Статус |
|---|---|---|
| Пайплайн запуску (version.json, бібліотеки, natives, assets) | `packages/app-lib/src/launcher/` | готово |
| Microsoft OAuth (MSA→XBL→XSTS→MC) | `packages/app-lib/src/state/minecraft_auth.rs` | готово |
| Встановлення лоадерів Fabric/Forge/**NeoForge**/Quilt | `ModLoader` @ `state/instance_types.rs:81` | готово |
| Список усіх версій MC і версій лоадерів | `api/metadata.rs` → `get_minecraft_versions()` / `get_loader_versions(loader)` | готово |
| Керування Java-рантаймами | `api/jre.rs` | готово |
| Кеш метаданих у SQLite | `state/cache.rs` | готово |

**Вимога №5 закрита upstream'ом.** Мультиверсійність — це `daedalus`-метадані: `MODRINTH_LAUNCHER_META_URL`
(compile-time `env!`, див. `state/cache.rs:1521`). 1.21.1/NeoForge — просто одне зі значень
`ModLoader::NeoForge` + версія. Нічого спеціального під 1.21.1 закладати не треба.

> Рішення: meta-URL поки лишаємо на Modrinth. Якщо захочемо незалежності — піднімаємо свій
> `apps/daedalus_client` і міняємо env-змінну. Це не блокує старт.

## Що дописуємо

### 1. Офлайн-акаунти

Точки втручання (перевірено в коді):

- `state/minecraft_auth.rs:201` — `struct Credentials { offline_profile, access_token, refresh_token, expires, active }`.
  Додаємо ознаку офлайн-акаунта (окреме поле / варіант), UUID рахуємо як vanilla: `UUIDv3("OfflinePlayer:<нік>")`.
- `state/minecraft_auth.rs:111,142` — поряд з `login_begin`/`login_finish` додаємо `login_offline(username)`.
- `launcher/args.rs:354` — `${user_type}` захардкоджено як `"msa"`; для офлайну → `"legacy"`.
- `api/instance/run.rs:242` — `accessToken` у launch-профілі; для офлайну — плейсхолдер.
- `Credentials::maybe_online_profile()` для офлайн-акаунта повертає `None`, і нік уже коректно
  береться з `offline_profile.name` — окремої правки не потребує.

Сервер має працювати з `online-mode=false`. Щоб офлайн-гравці не займали ніки ліцензійних і мали
скіни — на сервері `authlib-injector` + власний Yggdrasil (drasl / Ely.by).

### 2. Sync-движок збірки (ядро продукту)

Стан інстансу: `.terrarium/state.json` — для кожного файлу зберігаємо **політику** і
`installedHash` («яким лаунчер його поклав»).

Оновлення = порівняння трьох станів: попередній маніфест ↔ реальний диск ↔ новий маніфест.

| Ситуація | Дія |
|---|---|
| Є в новому маніфесті, немає на диску | завантажити |
| `managed` (моди пака), хеш ≠ цільового | замінити; якщо гравець правив — спершу в `.terrarium/backup/` |
| `seed` (конфіги, `options.txt`, кейбінди) | покласти **лише якщо відсутній**; далі не чіпати ніколи |
| `merge` | якщо `диск == installedHash` → перезаписати; інакше 3-way merge (TOML/JSON/properties) |
| Був у старому маніфесті, немає в новому | видалити **тільки якщо** `диск == installedHash`; інакше в бекап + повідомити |
| **Немає в жодному маніфесті** | **не чіпати ніколи** ← гарантія для клієнтських модів гравця |

Додатково:
- `mods/X.jar.disabled` → гравець свідомо вимкнув мод пака: не качати назад, лише показати в UI.
- Опційні моди пака (шейдери, міні-мапи): вибір гравця живе в стані лаунчера й переживає оновлення.

### 3. Доставка через GitHub Releases

Моди у свої релізи **не кладемо** — тягнемо з Modrinth/CurseForge CDN за хешем (як `.mrpack`).

```
terrarium-pack (public)
└── Release v1.4.2
    ├── manifest.json    # файли: hash + url + policy; підписаний ed25519
    └── overrides.zip    # наші конфіги, ресурспаки, скрипти
```

- Вказівник на останню версію — `releases/latest/download/manifest.json` (стабільний редірект,
  без лімітів). **GitHub API не використовуємо**: 60 запитів/год на IP → гравці за спільним NAT
  отримають 403.
- Ліміти: 2 ГБ на асет; трафік Releases для публічних репо не тарифікується. **Git LFS не чіпаємо.**
- Пак робимо **Modrinth-first**: частина модів на CurseForge має вимкнені сторонні завантаження → 403.
- Публічний ключ підпису вшитий у лаунчер. Без підпису компрометація репо = RCE в усіх гравців.
- Самооновлення лаунчера — Tauri updater; маніфест `releases/latest/download/latest.json` у цьому ж репо. Процес: `docs-terrarium/RELEASE.md`.

## Юридичне

- `apps/app` і `apps/app-frontend` — **GPL-3.0-only**. Наш форк має лишатися відкритим.
- `packages/app-lib` (крейт `theseus`) — **без файлу ліцензії й без поля `license`**.
  Найбезпечніше трактувати як GPL-3 разом з рештою застосунку.
- `COPYING.md`: брендинг Modrinth треба **видалити з форку** — логотипи, обкладинки, `.idea/icon.svg`.

## Тулчейн (Windows)

| Компонент | Статус |
|---|---|
| Node 24.13.1, npm 11.8.0 | є |
| pnpm | встановлено |
| Java 21 (OpenJDK) | є |
| Visual Studio Community 2026 | є, але **без workload «Desktop development with C++»** |
| WebView2 152.x | є |
| Rust 1.95.0 (пін у `rust-toolchain.toml`) | встановлюється |

`.cargo/config.toml` використовує `rust-lld` як лінкер, але libs Windows SDK/MSVC усе одно потрібні —
без C++ workload складання не піде.
