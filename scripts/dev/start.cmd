@echo off
rem Запуск Terrarium Launcher у dev-режимі (гаряче перезавантаження фронтенду,
rem автоперезбірка Rust). Закрити вікно лаунчера = зупинити все.
cd /d "%~dp0..\.."
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
title Terrarium Launcher - dev
call :cleanup
pnpm app:dev
call :cleanup
pause
exit /b

:cleanup
rem Залишки попереднього запуску. Windows не вбиває дочірні процеси
rem pnpm -> turbo -> vite, коли tauri dev завершується, тож старий vite лишається
rem на порту 1420; новий vite (strictPort) тоді падає, tauri dev виходить з кодом 1
rem і через кілька секунд закриває щойно відкритий лаунчер. Стара dev-збірка
rem лаунчера теж заважає: single-instance просто передає їй фокус.
for /f "tokens=5" %%p in ('netstat -ano ^| findstr /r /c:":1420 .*LISTENING"') do (
  echo [dev] Закриваю старий vite на порту 1420 ^(pid %%p^)
  taskkill /f /pid %%p >nul 2>&1
)
powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "Get-Process theseus_gui -ErrorAction SilentlyContinue | Where-Object { $_.Path -like '*\target\debug\*' } | ForEach-Object { Write-Host \"[dev] Закриваю стару dev-збірку лаунчера (pid $($_.Id))\"; Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue }"
exit /b
