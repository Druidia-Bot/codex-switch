# Builds the release binaries and zips them with the installer into dist\.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $root
$env:PATH = ($env:PATH -split ';' | Where-Object { $_ -notmatch 'Git\\usr\\bin' }) -join ';'
# cargo writes progress to stderr, which Windows PowerShell 5.1 treats as an
# error under 'Stop'; relax it for the build only.
$ErrorActionPreference = 'Continue'
cargo +stable-x86_64-pc-windows-msvc build --release 2>&1 | ForEach-Object { "$_" }
$ErrorActionPreference = 'Stop'
if ($LASTEXITCODE -ne 0) { throw 'build failed' }
$stage = Join-Path $root 'dist\codex-switch-windows-x64'
Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $stage | Out-Null
Copy-Item target\release\codex-switch.exe, target\release\codex-switch-bar.exe, scripts\install-start-menu.ps1, README.md $stage
$zip = "$stage.zip"
Remove-Item $zip -ErrorAction SilentlyContinue
Compress-Archive -Path "$stage\*" -DestinationPath $zip
Write-Output "wrote $zip"
