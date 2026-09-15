<#
.SYNOPSIS
  Збирає Terrarium Launcher з увімкненим оновлювачем і публікує реліз на GitHub.

.DESCRIPTION
  1. Збирає фронтенд (окремо, бо вкладений turbo з-під `tauri build` падає на Windows).
  2. `tauri build` з tauri-release.conf.json: фіча `updater`, підпис .sig нашим ключем.
  3. Генерує latest.json (формат Tauri updater) і створює реліз через `gh`.

  Версія береться з apps/app-frontend/package.json — підніми її перед релізом,
  інакше оновлювач у гравців не побачить нову версію.

.PARAMETER Notes
  Текст release notes (markdown). Якщо не задано — береться з файлу -NotesFile або авто-текст.
.PARAMETER SkipBuild
  Не збирати, лише опублікувати вже зібраний інсталятор з target/release/bundle/nsis.
.PARAMETER Draft
  Створити реліз як чернетку (оновлювач її не побачить, поки не опублікуєш).

.EXAMPLE
  $env:TAURI_SIGNING_PRIVATE_KEY = "$env:USERPROFILE\.tauri\terrarium-launcher.key"
  .\scripts\terrarium-release.ps1 -NotesFile .\release-notes.md
#>
[CmdletBinding()]
param(
	[string]$Notes,
	[string]$NotesFile,
	[switch]$SkipBuild,
	[switch]$Draft,
	[string]$Repo = 'Kemzino/TerrariumLauncher'
)

$ErrorActionPreference = 'Stop'
$root = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $root

# --- ключ підпису -----------------------------------------------------------
if (-not $env:TAURI_SIGNING_PRIVATE_KEY) {
	$defaultKey = Join-Path $env:USERPROFILE '.tauri\terrarium-launcher.key'
	if (-not (Test-Path $defaultKey)) {
		throw "Не знайдено ключ підпису. Задай TAURI_SIGNING_PRIVATE_KEY або поклади ключ у $defaultKey"
	}
	$env:TAURI_SIGNING_PRIVATE_KEY = $defaultKey
}
if ($null -eq $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) { $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = '' }

# --- версія -----------------------------------------------------------------
$version = (Get-Content 'apps/app-frontend/package.json' -Raw | ConvertFrom-Json).version
$tag = "v$version"
Write-Host "==> Terrarium Launcher $version ($tag)" -ForegroundColor Cyan

$releaseExists = $false
try { gh release view $tag -R $Repo --json tagName *> $null; $releaseExists = ($LASTEXITCODE -eq 0) } catch { $releaseExists = $false }
if ($releaseExists) {
	throw "Реліз $tag уже існує на GitHub. Підніми версію в apps/app-frontend/package.json"
}

# --- збірка -----------------------------------------------------------------
if (-not $SkipBuild) {
	Write-Host '==> Збірка фронтенду' -ForegroundColor Cyan
	pnpm turbo run build --filter=@modrinth/app-frontend
	if ($LASTEXITCODE -ne 0) { throw 'Збірка фронтенду впала' }

	Write-Host '==> tauri build (updater + підпис)' -ForegroundColor Cyan
	Push-Location apps/app
	try {
		pnpm exec tauri build --ci --config tauri-release.conf.json --config tauri.local-release.conf.json --bundles nsis
		if ($LASTEXITCODE -ne 0) { throw 'tauri build впав' }
	} finally { Pop-Location }
}

# --- артефакти --------------------------------------------------------------
$bundleDir = Join-Path $root 'target/release/bundle/nsis'
$setup = Get-ChildItem $bundleDir -Filter "*_${version}_x64-setup.exe" | Select-Object -First 1
if (-not $setup) { throw "Не знайдено інсталятор *_${version}_x64-setup.exe у $bundleDir" }
$sig = "$($setup.FullName).sig"
if (-not (Test-Path $sig)) { throw "Не знайдено підпис $sig — збірка йшла без TAURI_SIGNING_PRIVATE_KEY?" }

$outDir = Join-Path $root "dist-release/$tag"
New-Item -ItemType Directory -Force $outDir | Out-Null
# Імена без пробілів: GitHub перейменовує асети, а URL у latest.json має бути передбачуваним
$assetName = "Terrarium-Launcher_${version}_x64-setup.exe"
Copy-Item $setup.FullName (Join-Path $outDir $assetName) -Force
Copy-Item $sig (Join-Path $outDir "$assetName.sig") -Force

# --- release notes ----------------------------------------------------------
if (-not $Notes) {
	if ($NotesFile) { $Notes = Get-Content $NotesFile -Raw }
	else { $Notes = "Terrarium Launcher $version" }
}
$notesPath = Join-Path $outDir 'release-notes.md'
[IO.File]::WriteAllText($notesPath, $Notes, (New-Object Text.UTF8Encoding $false))

# --- latest.json (https://tauri.app/plugin/updater/#server-support) ----------
$manifest = [ordered]@{
	version  = $version
	notes    = $Notes
	pub_date = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
	platforms = [ordered]@{
		'windows-x86_64' = [ordered]@{
			signature = (Get-Content $sig -Raw).Trim()
			url       = "https://github.com/$Repo/releases/download/$tag/$assetName"
		}
	}
}
$latestPath = Join-Path $outDir 'latest.json'
[IO.File]::WriteAllText($latestPath, ($manifest | ConvertTo-Json -Depth 5), (New-Object Text.UTF8Encoding $false))
Write-Host "==> latest.json:" -ForegroundColor Cyan
Get-Content $latestPath

# --- реліз на GitHub --------------------------------------------------------
Write-Host "==> gh release create $tag" -ForegroundColor Cyan
$ghArgs = @('release', 'create', $tag, '-R', $Repo, '--title', "Terrarium Launcher $version", '--notes-file', $notesPath, '--target', (git rev-parse HEAD))
if ($Draft) { $ghArgs += '--draft' }
$ghArgs += @((Join-Path $outDir $assetName), (Join-Path $outDir "$assetName.sig"), $latestPath)
gh @ghArgs
if ($LASTEXITCODE -ne 0) { throw 'gh release create впав' }

Write-Host "==> Готово: https://github.com/$Repo/releases/tag/$tag" -ForegroundColor Green