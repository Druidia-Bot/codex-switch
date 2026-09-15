# Creates (or refreshes) a per-user Start Menu shortcut "Codex Switch" that
# launches codex-switch-bar.exe from ~/.local/bin. Re-run after rebuilding.
$ErrorActionPreference = 'Stop'
$exe = Join-Path $HOME '.local\bin\codex-switch-bar.exe'
if (-not (Test-Path -LiteralPath $exe)) { throw "codex-switch-bar.exe not found at $exe - build and copy it first (see README)." }
$programs = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
$lnk = Join-Path $programs 'Codex Switch.lnk'
$shell = New-Object -ComObject WScript.Shell
$s = $shell.CreateShortcut($lnk)
$s.TargetPath = $exe
$s.WorkingDirectory = Split-Path -Parent $exe
$s.Description = 'Switch Codex between OpenAI and OpenRouter'
$s.IconLocation = "$exe,0"
$s.Save()
Write-Output "Start Menu shortcut written: $lnk"
