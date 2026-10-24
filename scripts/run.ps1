# Windows equivalent of `make run`: 1 instance.
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\pnpm.ps1"
$pnpm = Get-Pnpm

New-Item -ItemType Directory -Force app/build | Out-Null
Set-Location app
& $pnpm tauri dev
exit $LASTEXITCODE
