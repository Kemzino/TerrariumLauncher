-- Terrarium: кеш CurseForge — відбитки файлів (за sha1), збіги відбиток → файл
-- мода, і картки модів (назва, іконка, автори, останні файли для оновлень).
-- Один K/V на всі види: `kind` = fingerprint | match | mod.
CREATE TABLE terrarium_curseforge_cache (
	kind TEXT NOT NULL,
	key TEXT NOT NULL,
	data TEXT NOT NULL,
	expires INTEGER NOT NULL,
	PRIMARY KEY (kind, key)
);
