# codex-switch installer (Windows).
#
# Run this from the extracted release folder (or from a source checkout after
# `cargo build --release`). It copies codex-switch.exe and codex-switch-bar.exe
# into ~\.local\bin, adds that folder to your user PATH if needed, and creates a
# Start Menu entry called "Codex Switch". Safe to re-run after an update.
#
#   Right-click -> "Run with PowerShell", or:
#   powershell -ExecutionPolicy Bypass -File .\install-start-menu.ps1
$ErrorActionPreference = 'Stop'

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$candidates = @($here, (Join-Path $here '..\target\release'), (Join-Path $here 'target\release'))
$source = $candidates | Where-Object { Test-Path (Join-Path $_ 'codex-switch-bar.exe') } | Select-Object -First 1
if (-not $source) { throw "codex-switch-bar.exe not found next to this script. Extract the release zip fully, or build with cargo first." }

$bin = Join-Path $HOME '.local\bin'
New-Item -ItemType Directory -Force $bin | Out-Null
foreach ($name in 'codex-switch.exe', 'codex-switch-bar.exe') {
    $src = Join-Path $source $name
    if (Test-Path $src) {
        # A running bar holds its exe open; stop it and wait for the lock to clear.
        $proc = [IO.Path]::GetFileNameWithoutExtension($name)
        Get-Process -Name $proc -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
        for ($i = 0; $i -lt 20 -and (Get-Process -Name $proc -ErrorAction SilentlyContinue); $i++) { Start-Sleep -Milliseconds 150 }
        $copied = $false
        for ($i = 0; $i -lt 10 -and -not $copied; $i++) {
            try { Copy-Item $src (Join-Path $bin $name) -Force; $copied = $true } catch { Start-Sleep -Milliseconds 300 }
        }
        if (-not $copied) { throw "could not replace $name - close Codex Switch and run this again." }
        Unblock-File (Join-Path $bin $name) -ErrorAction SilentlyContinue
        Write-Output "installed $name -> $bin"
    }
}

$userPath = [Environment]::GetEnvironmentVariable('PATH', 'User')
if (($userPath -split ';') -notcontains $bin) {
    [Environment]::SetEnvironmentVariable('PATH', ($userPath.TrimEnd(';') + ';' + $bin), 'User')
    Write-Output "added $bin to your user PATH (new terminals will see 'codex-switch')"
}

$exe = Join-Path $bin 'codex-switch-bar.exe'
$programs = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
$lnk = Join-Path $programs 'Codex Switch.lnk'
$shell = New-Object -ComObject WScript.Shell
$s = $shell.CreateShortcut($lnk)
$s.TargetPath = $exe
$s.WorkingDirectory = $bin
$s.Description = 'Switch Codex between OpenAI and OpenRouter'
$s.IconLocation = "$exe,0"
$s.Save()
Write-Output "Start Menu entry created: Codex Switch"
Write-Output "Done. Press the Windows key and type 'Codex Switch' to open it."
