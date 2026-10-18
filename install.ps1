# Music App installer for Windows (macOS / Linux: install.sh).
#
#   .\install.ps1                      client from the latest GitHub release (default)
#   .\install.ps1 -Mode server         group server only (Docker), no client
#   .\install.ps1 -Mode both           server, then client
#   .\install.ps1 -FromSource          build from a clone instead of downloading a release
#
# Without a clone: irm https://raw.githubusercontent.com/OverCapi/music_app/master/install.ps1 -OutFile install.ps1; .\install.ps1 -Mode server
[CmdletBinding()]
param(
    [ValidateSet('client', 'server', 'both')][string]$Mode = 'client',
    [switch]$FromSource,
    [string]$Version = ''
)
$ErrorActionPreference = 'Stop'

$Repo = 'OverCapi/music_app'
$Image = if ($env:MUSIC_SERVER_IMAGE) { $env:MUSIC_SERVER_IMAGE } else { 'ghcr.io/overcapi/music-group-server:latest' }
$Container = 'music-group-server'
$Summary = @()

function Say($msg) { Write-Host "==> $msg" -ForegroundColor Cyan }
function Have($cmd) { [bool](Get-Command $cmd -ErrorAction SilentlyContinue) }
function Refresh-Path {
    $env:Path = [Environment]::GetEnvironmentVariable('Path', 'Machine') + ';' + [Environment]::GetEnvironmentVariable('Path', 'User')
}
function Winget-Install($id) {
    winget install --id $id -e --silent --accept-package-agreements --accept-source-agreements
    # 0x8A150061 / "already installed" is not a failure.
    if ($LASTEXITCODE -ne 0 -and $LASTEXITCODE -ne -1978335189) { throw "winget a échoué pour $id (code $LASTEXITCODE)" }
    Refresh-Path
}

if (-not (Have winget)) { throw "winget est requis (Windows 10 1809+ / 11 : installe 'App Installer' depuis le Microsoft Store)" }

function Repo-Dir {
    $here = $PSScriptRoot
    if ($here -and (Test-Path (Join-Path $here 'app\src-tauri\tauri.conf.json'))) { return $here }
    return $null
}

# ---------------------------------------------------------------- client ----

function Install-RuntimeDeps {
    Say 'Dépendances : yt-dlp, ffmpeg, node'
    if (-not (Have yt-dlp)) { Winget-Install 'yt-dlp.yt-dlp' }
    if (-not (Have ffmpeg)) { Winget-Install 'Gyan.FFmpeg' }
    if (-not (Have node)) { Winget-Install 'OpenJS.NodeJS.LTS' }
}

function Install-ClientRelease {
    $api = "https://api.github.com/repos/$Repo/releases/latest"
    if ($Version) { $api = "https://api.github.com/repos/$Repo/releases/tags/$Version" }
    $release = Invoke-RestMethod $api
    $asset = $release.assets | Where-Object { $_.name -match '_x64-setup\.exe$' } | Select-Object -First 1
    if (-not $asset) { throw "aucune release Windows trouvée sur github.com/$Repo/releases (essaie -FromSource)" }
    $tmp = Join-Path ([IO.Path]::GetTempPath()) $asset.name
    Say "Téléchargement de $($asset.name)"
    Invoke-WebRequest $asset.browser_download_url -OutFile $tmp
    Say 'Installation (silencieuse)'
    Start-Process $tmp -ArgumentList '/S' -Wait
    Remove-Item $tmp -ErrorAction SilentlyContinue
    $script:Summary += "App : installée (menu Démarrer : Music App)"
}

function Install-ClientSource {
    Say 'Dépendances de compilation'
    if (-not (Have cargo)) { Winget-Install 'Rustlang.Rustup' }
    if (-not (Have cmake)) { Winget-Install 'Kitware.CMake' }
    if (-not (Have git)) { Winget-Install 'Git.Git' }
    # MSVC build tools (C++ workload) are required by Rust on Windows.
    winget install --id Microsoft.VisualStudio.2022.BuildTools -e --silent --accept-package-agreements --accept-source-agreements --override '--passive --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended'
    Refresh-Path
    if (-not (Have pnpm)) { corepack enable pnpm; if (-not (Have pnpm)) { npm install -g pnpm } }
    $dir = Repo-Dir
    if (-not $dir) {
        $dir = Join-Path ([IO.Path]::GetTempPath()) 'music_app'
        Say "Clone de $Repo"
        git clone --depth 1 "https://github.com/$Repo.git" $dir
    }
    Say 'Compilation (quelques minutes)'
    New-Item -ItemType Directory -Force (Join-Path $dir 'app\build') | Out-Null
    Push-Location (Join-Path $dir 'app')
    try {
        pnpm install --frozen-lockfile
        pnpm tauri build --bundles nsis
        $setup = Get-ChildItem (Join-Path $dir 'target\release\bundle\nsis') -Filter '*-setup.exe' | Select-Object -First 1
        Start-Process $setup.FullName -ArgumentList '/S' -Wait
    } finally { Pop-Location }
    $script:Summary += "App : installée (menu Démarrer : Music App)"
}

function Install-Client {
    Install-RuntimeDeps
    if ($FromSource) { Install-ClientSource } else { Install-ClientRelease }
}

# ---------------------------------------------------------------- server ----

function Install-Server {
    if (-not (Have docker)) {
        Say 'Installation de Docker Desktop'
        Winget-Install 'Docker.DockerDesktop'
        Write-Host 'Docker Desktop est installé : lance-le une fois (et accepte les conditions), puis relance ce script.' -ForegroundColor Yellow
        if (-not (Have docker)) { exit 1 }
    }
    docker info *> $null
    if ($LASTEXITCODE -ne 0) {
        Start-Process "$env:ProgramFiles\Docker\Docker\Docker Desktop.exe" -ErrorAction SilentlyContinue
        for ($i = 0; $i -lt 60; $i++) { docker info *> $null; if ($LASTEXITCODE -eq 0) { break }; Start-Sleep 2 }
        if ($LASTEXITCODE -ne 0) { throw 'le démon Docker ne répond pas' }
    }
    $dir = Repo-Dir
    if ($FromSource -and $dir) {
        Say 'Serveur : docker compose up --build'
        Push-Location $dir
        try { docker compose up -d --build } finally { Pop-Location }
    } else {
        Say "Serveur : $Image"
        docker pull $Image
        docker rm -f $Container *> $null
        docker run -d --name $Container --restart unless-stopped -p 8787:8787 -v music-server-data:/data $Image | Out-Null
    }
    $ip = (Get-NetIPAddress -AddressFamily IPv4 | Where-Object { $_.IPAddress -notmatch '^(127|169\.254)\.' } | Select-Object -First 1).IPAddress
    if (-not $ip) { $ip = 'localhost' }
    $script:Summary += "Serveur : http://${ip}:8787 (à saisir dans « Groupe d'écoute »)"
}

# ------------------------------------------------------------------ main ----

switch ($Mode) {
    'client' { Install-Client }
    'server' { Install-Server }
    'both' { Install-Server; Install-Client }
}

Say 'Terminé'
$Summary | ForEach-Object { Write-Host "  $_" }
