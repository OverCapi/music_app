# Dot-source to get Get-Pnpm: returns a runnable pnpm (.cmd/.exe, usable with Start-Process),
# even when the PATH inherited from make is stale or incomplete.
function Get-Pnpm {
    $found = Get-Command pnpm.cmd, pnpm.exe -CommandType Application -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if ($found) { return $found.Source }

    $candidates = @(
        "$env:APPDATA\npm\pnpm.cmd",       # npm i -g pnpm
        "$env:LOCALAPPDATA\pnpm\pnpm.exe", # standalone installer
        "$env:ProgramFiles\nodejs\pnpm.cmd" # corepack enable
    )
    foreach ($c in $candidates) {
        if (Test-Path $c) { return $c }
    }

    Write-Host "pnpm introuvable. Cherche dans le PATH puis dans:" -ForegroundColor Red
    $candidates | ForEach-Object { Write-Host "  $_" }
    Write-Host "Installe-le avec: npm i -g pnpm" -ForegroundColor Yellow
    exit 1
}
