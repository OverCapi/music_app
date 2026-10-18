#!/usr/bin/env bash
# Music App installer for macOS and Linux (Windows: install.ps1).
#
#   ./install.sh                     client from the latest GitHub release (default)
#   ./install.sh --server            group server only (Docker), no client
#   ./install.sh --both              server, then client
#   ./install.sh --from-source       build from a clone instead of downloading a release
#
# Without a clone: curl -fsSL https://raw.githubusercontent.com/OverCapi/music_app/master/install.sh | bash -s -- --server
set -euo pipefail

REPO="OverCapi/music_app"
IMAGE="${MUSIC_SERVER_IMAGE:-ghcr.io/overcapi/music-group-server:latest}"
CONTAINER="music-group-server"
MODE="client"
FROM_SOURCE=0
VERSION=""
SUMMARY=()

usage() {
  cat <<'EOF'
Usage: install.sh [--client | --server | --both] [--from-source] [--version vX.Y.Z]

  --client        install the app (default); never installs Docker or the server
  --server        install and start the group server in Docker; never installs the app
  --both          server, then client
  --from-source   build from source (the current clone, else a fresh one) instead of
                  downloading the release (server: docker compose --build)
  --version TAG   release to install instead of the latest (e.g. a pre-release)
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --client) MODE=client ;;
    --server) MODE=server ;;
    --both) MODE=both ;;
    --from-source) FROM_SOURCE=1 ;;
    --version) VERSION="${2:?--version needs a tag}"; shift ;;
    -h | --help) usage; exit 0 ;;
    *) echo "Option inconnue : $1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

say() { printf '\033[1m==> %s\033[0m\n' "$*"; }
die() { printf 'Erreur : %s\n' "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }
as_root() { if [ "$(id -u)" -eq 0 ]; then "$@"; else sudo "$@"; fi; }

OS="$(uname -s)"
ARCH="$(uname -m)"
case "$OS" in Darwin | Linux) ;; *) die "système non supporté : $OS (Windows : install.ps1)" ;; esac

PM=""
if [ "$OS" = Darwin ]; then
  PM=brew
elif have apt-get; then PM=apt
elif have dnf; then PM=dnf
elif have pacman; then PM=pacman
fi

# Directory of a clone, if the script runs from one.
repo_dir() {
  local here
  here="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" 2>/dev/null && pwd || true)"
  if [ -n "$here" ] && [ -f "$here/app/src-tauri/tauri.conf.json" ]; then echo "$here"; fi
}

# Installs system packages: pkg_install <apt names> -- <dnf names> -- <pacman names>
pkg_install() {
  local apt="$1" dnf="$2" pacman="$3"
  case "$PM" in
    apt) as_root apt-get update -qq && as_root apt-get install -y $apt ;;
    dnf) as_root dnf install -y $dnf ;;
    pacman) as_root pacman -S --needed --noconfirm $pacman ;;
    *) die "gestionnaire de paquets non reconnu : installe à la main ($apt)" ;;
  esac
}

ensure_brew() {
  have brew && return
  [ -x /opt/homebrew/bin/brew ] && { eval "$(/opt/homebrew/bin/brew shellenv)"; return; }
  [ -x /usr/local/bin/brew ] && { eval "$(/usr/local/bin/brew shellenv)"; return; }
  say "Installation de Homebrew"
  /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
  [ -x /opt/homebrew/bin/brew ] && eval "$(/opt/homebrew/bin/brew shellenv)"
  [ -x /usr/local/bin/brew ] && eval "$(/usr/local/bin/brew shellenv)"
  have brew || die "Homebrew introuvable après installation"
}

# ---------------------------------------------------------------- client ----

install_runtime_deps() {
  say "Dépendances : yt-dlp, ffmpeg, node"
  if [ "$OS" = Darwin ]; then
    ensure_brew
    brew install yt-dlp ffmpeg node
    return
  fi
  case "$PM" in
    apt) pkg_install "ffmpeg nodejs curl ca-certificates" "" "" ;;
    dnf) pkg_install "" "ffmpeg-free nodejs curl" "" ;;
    pacman) pkg_install "" "" "ffmpeg nodejs curl" ;;
    *) die "installe ffmpeg, node et curl à la main" ;;
  esac
  # Best effort: ALSA runtime for audio output (package name depends on the release).
  case "$PM" in
    apt) as_root apt-get install -y libasound2t64 2>/dev/null || as_root apt-get install -y libasound2 || true ;;
    dnf) as_root dnf install -y alsa-lib || true ;;
  esac
  # Distro yt-dlp packages are too old for YouTube: use the official standalone binary.
  local asset=yt-dlp_linux
  [ "$ARCH" = aarch64 ] || [ "$ARCH" = arm64 ] && asset=yt-dlp_linux_aarch64
  mkdir -p "$HOME/.local/bin"
  curl -fsSL "https://github.com/yt-dlp/yt-dlp/releases/latest/download/$asset" -o "$HOME/.local/bin/yt-dlp"
  chmod +x "$HOME/.local/bin/yt-dlp"
}

release_url() { # release_url <regex on the asset name>
  local api="https://api.github.com/repos/$REPO/releases/latest"
  [ -n "$VERSION" ] && api="https://api.github.com/repos/$REPO/releases/tags/$VERSION"
  curl -fsSL "$api" | grep -o '"browser_download_url": *"[^"]*"' | sed 's/.*"\(http[^"]*\)"/\1/' | grep -E "$1" | head -n1
}

install_linux_desktop_entry() { # install_linux_desktop_entry <exec> <icon>
  local dir="$HOME/.local/share/applications"
  mkdir -p "$dir"
  cat >"$dir/music-app.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Music App
Comment=Desktop music player
Exec=$1
Icon=$2
Categories=AudioVideo;Audio;Player;
Terminal=false
EOF
}

install_client_release() {
  local tmp url
  tmp="$(mktemp -d)"
  if [ "$OS" = Darwin ]; then
    case "$ARCH" in arm64) url="$(release_url '_aarch64\.dmg$')" ;; *) url="$(release_url '_x64\.dmg$')" ;; esac
    [ -n "$url" ] || die "aucune release macOS trouvée sur github.com/$REPO/releases (essaie --from-source)"
    say "Téléchargement de $(basename "$url")"
    curl -fL "$url" -o "$tmp/app.dmg"
    hdiutil attach -nobrowse -quiet -mountpoint "$tmp/mnt" "$tmp/app.dmg"
    rm -rf "/Applications/Music App.app"
    cp -R "$tmp/mnt/Music App.app" /Applications/
    hdiutil detach -quiet "$tmp/mnt"
    # Not notarized: drop the quarantine flag so the first launch is not blocked.
    xattr -dr com.apple.quarantine "/Applications/Music App.app"
    SUMMARY+=("App : /Applications/Music App.app")
  else
    [ "$ARCH" = x86_64 ] || die "pas de release Linux pour $ARCH (essaie --from-source)"
    url="$(release_url '_amd64\.AppImage$')"
    [ -n "$url" ] || die "aucune release Linux trouvée sur github.com/$REPO/releases (essaie --from-source)"
    say "Téléchargement de $(basename "$url")"
    curl -fL "$url" -o "$tmp/app.AppImage"
    chmod +x "$tmp/app.AppImage"
    # Extracted instead of run as an AppImage: no FUSE needed.
    local dest="$HOME/.local/share/music-app"
    rm -rf "$dest" && mkdir -p "$dest" "$HOME/.local/bin"
    (cd "$tmp" && ./app.AppImage --appimage-extract >/dev/null)
    mv "$tmp/squashfs-root/"* "$dest/"
    ln -sf "$dest/AppRun" "$HOME/.local/bin/music-app"
    local icon
    icon="$(find "$dest" -maxdepth 1 -name '*.png' | head -n1)"
    install_linux_desktop_entry "$dest/AppRun" "${icon:-music-app}"
    SUMMARY+=("App : $HOME/.local/bin/music-app (et menu des applications)")
  fi
  rm -rf "$tmp"
}

install_build_deps() {
  say "Dépendances de compilation"
  if [ "$OS" = Darwin ]; then
    xcode-select -p >/dev/null 2>&1 || { xcode-select --install || true; die "termine l'installation des Xcode CLT puis relance"; }
    ensure_brew
    brew install cmake pnpm
  else
    pkg_install \
      "build-essential curl wget file git cmake pkg-config libssl-dev libwebkit2gtk-4.1-dev libxdo-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev" \
      "gcc gcc-c++ make curl wget file git cmake pkgconf-pkg-config openssl-devel webkit2gtk4.1-devel libxdo-devel libappindicator-gtk3-devel librsvg2-devel alsa-lib-devel" \
      "base-devel curl wget file git cmake openssl webkit2gtk-4.1 xdotool libayatana-appindicator librsvg alsa-lib"
    have pnpm || corepack enable pnpm 2>/dev/null || as_root npm install -g pnpm
  fi
  if ! have cargo; then
    say "Installation de Rust"
    curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal
  fi
  [ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
}

install_client_source() {
  install_build_deps
  local dir
  dir="$(repo_dir)"
  if [ -z "$dir" ]; then
    dir="$(mktemp -d)/music_app"
    say "Clone de $REPO"
    git clone --depth 1 "https://github.com/$REPO.git" "$dir"
  fi
  say "Compilation (quelques minutes)"
  mkdir -p "$dir/app/build"
  (cd "$dir/app" && pnpm install --frozen-lockfile && pnpm tauri build $([ "$OS" = Darwin ] && echo "--bundles app" || echo "--no-bundle"))
  if [ "$OS" = Darwin ]; then
    rm -rf "/Applications/Music App.app"
    cp -R "$dir/target/release/bundle/macos/Music App.app" /Applications/
    SUMMARY+=("App : /Applications/Music App.app")
  else
    mkdir -p "$HOME/.local/bin" "$HOME/.local/share/icons"
    install -m 755 "$dir/target/release/music_app" "$HOME/.local/bin/music-app"
    cp "$dir/app/src-tauri/icons/128x128.png" "$HOME/.local/share/icons/music-app.png"
    install_linux_desktop_entry "$HOME/.local/bin/music-app" "$HOME/.local/share/icons/music-app.png"
    SUMMARY+=("App : $HOME/.local/bin/music-app (et menu des applications)")
  fi
}

install_client() {
  install_runtime_deps
  if [ "$FROM_SOURCE" = 1 ]; then install_client_source; else install_client_release; fi
}

# ---------------------------------------------------------------- server ----

# docker, or sudo docker when the user is not in the docker group yet.
dk() { if docker info >/dev/null 2>&1; then docker "$@"; else as_root docker "$@"; fi; }

ensure_docker() {
  if have docker; then
    :
  elif [ "$OS" = Darwin ]; then
    ensure_brew
    say "Installation d'OrbStack (Docker)"
    brew install --cask orbstack
  else
    say "Installation de Docker (script officiel get.docker.com)"
    curl -fsSL https://get.docker.com | as_root sh
    as_root systemctl enable --now docker 2>/dev/null || true
  fi
  if [ "$OS" = Darwin ] && ! docker info >/dev/null 2>&1; then
    open -a OrbStack 2>/dev/null || open -a Docker 2>/dev/null || true
  fi
  local i
  for i in $(seq 60); do
    dk info >/dev/null 2>&1 && return
    sleep 2
  done
  die "le démon Docker ne répond pas"
}

install_server() {
  ensure_docker
  local dir
  dir="$(repo_dir)"
  if [ "$FROM_SOURCE" = 1 ] && [ -n "$dir" ]; then
    say "Serveur : docker compose up --build"
    (cd "$dir" && dk compose up -d --build)
  else
    [ "$FROM_SOURCE" = 1 ] && say "Pas de clone ici : image publiée utilisée"
    say "Serveur : $IMAGE"
    dk pull "$IMAGE"
    dk rm -f "$CONTAINER" >/dev/null 2>&1 || true
    dk run -d --name "$CONTAINER" --restart unless-stopped \
      -p 8787:8787 -v music-server-data:/data "$IMAGE" >/dev/null
  fi
  local ip
  ip="$(hostname -I 2>/dev/null | awk '{print $1}')"
  [ -n "$ip" ] || ip="$(ipconfig getifaddr en0 2>/dev/null || echo localhost)"
  SUMMARY+=("Serveur : http://$ip:8787 (à saisir dans « Groupe d'écoute »)")
}

# ------------------------------------------------------------------ main ----

case "$MODE" in
  client) install_client ;;
  server) install_server ;;
  both) install_server; install_client ;;
esac

say "Terminé"
printf '  %s\n' "${SUMMARY[@]}"
