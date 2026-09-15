@echo off
rem Запуск Terrarium Launcher у dev-режимі (гаряче перезавантаження фронтенду,
rem автоперезбірка Rust). Закрити вікно лаунчера = зупинити все.
cd /d "%~dp0..\.."
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
title Terrarium Launcher - dev
pnpm app:dev
pause
