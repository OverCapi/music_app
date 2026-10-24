# Windows equivalent of `make run2`: 2 instances + group server in Docker.
# Ctrl-C stops everything (data kept in the volume).
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\pnpm.ps1"
$pnpm = Get-Pnpm
New-Item -ItemType Directory -Force app/build | Out-Null

function Wait-Port($port) {
    while (-not (Test-NetConnection 127.0.0.1 -Port $port -InformationLevel Quiet -WarningAction SilentlyContinue)) {
        Start-Sleep -Seconds 1
    }
}

cargo build -p music_app
docker compose up -d --build

$procs = @()
try {
    Wait-Port 8787
    $procs += Start-Process $pnpm -ArgumentList 'tauri', 'dev' -WorkingDirectory app -PassThru -NoNewWindow
    Wait-Port 1420
    $env:MUSIC_APP_PROFILE = 'b'
    $procs += Start-Process target/debug/music_app.exe -PassThru -NoNewWindow
    $procs | Wait-Process
}
finally {
    foreach ($p in $procs) {
        if (-not $p.HasExited) { taskkill /PID $p.Id /T /F | Out-Null }
    }
    docker compose down
}
