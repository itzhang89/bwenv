#!/usr/bin/env bash
# Install the latest bwenv binary from GitHub Releases (macOS / Linux).
# Windows: use https://github.com/itzhang89/bwenv/releases/latest and download the .zip.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/itzhang89/bwenv/main/install.sh | bash
#
# Optional environment:
#   BWENV_INSTALL_DIR   install directory (default: $HOME/.local/bin)
#   BWENV_GITHUB_REPO   owner/repo      (default: itzhang89/bwenv)

set -euo pipefail

REPO="${BWENV_GITHUB_REPO:-itzhang89/bwenv}"
INSTALL_DIR="${BWENV_INSTALL_DIR:-$HOME/.local/bin}"

die() {
  echo "install.sh: $*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"
}

detect_suffix() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os" in
    Linux)
      case "$arch" in
        x86_64) echo "linux-x64.tar.gz" ;;
        aarch64 | arm64) die "no prebuilt binary for Linux arm64 yet; build from source or open an issue" ;;
        *) die "unsupported Linux arch: $arch" ;;
      esac
      ;;
    Darwin)
      case "$arch" in
        arm64) echo "darwin-arm64.tar.gz" ;;
        x86_64) echo "darwin-x64.tar.gz" ;;
        *) die "unsupported macOS arch: $arch" ;;
      esac
      ;;
    *)
      die "unsupported OS: $os (use Windows builds from the Releases page)"
      ;;
  esac
}

pick_asset_url() {
  local json_file="$1"
  local pattern="$2"
  python3 -c "
import json, fnmatch, sys
with open(sys.argv[1], encoding='utf-8') as f:
    r = json.load(f)
pat = sys.argv[2]
for a in r.get('assets', []):
    n = a.get('name') or ''
    if fnmatch.fnmatch(n, pat) and a.get('browser_download_url'):
        print(a['browser_download_url'])
        sys.exit(0)
sys.exit(1)
" "$json_file" "$pattern" || return 1
}

main() {
  need_cmd curl
  need_cmd python3
  need_cmd tar
  need_cmd shasum

  local suffix
  suffix="$(detect_suffix)"
  local pattern="bwenv-*-${suffix}"

  echo "Fetching latest release from https://github.com/${REPO} ..."
  local work
  work="$(mktemp -d)"
  trap 'rm -rf "$work"' EXIT

  local json_file="${work}/release.json"
  curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" -o "$json_file" ||
    die "could not reach GitHub API (network or rate limit). Open https://github.com/${REPO}/releases/latest and download manually."

  local url sha_url
  url="$(pick_asset_url "$json_file" "$pattern")" ||
    die "no matching asset for ${pattern} in the latest release. See https://github.com/${REPO}/releases/latest"

  local sha_pattern="bwenv-*-${suffix}.sha256"
  sha_url="$(pick_asset_url "$json_file" "$sha_pattern" || true)"

  local fname
  fname="$(basename "$url")"
  local archive="${work}/${fname}"
  echo "Downloading ${fname} ..."
  curl -fsSL -o "$archive" "$url"

  if [[ -n "${sha_url:-}" ]]; then
    echo "Verifying SHA256 ..."
    curl -fsSL -o "${archive}.sha256" "$sha_url"
    (cd "$work" && shasum -a 256 -c "${fname}.sha256")
  else
    echo "Warning: no .sha256 asset found; skipping checksum." >&2
  fi

  tar -xzf "$archive" -C "$work"
  local bin="${work}/bwenv"
  [[ -f "$bin" ]] || die "archive did not contain bwenv"

  chmod +x "$bin"
  mkdir -p "$INSTALL_DIR"

  if [[ -w "$INSTALL_DIR" ]]; then
    mv -f "$bin" "${INSTALL_DIR}/bwenv"
  else
    need_cmd sudo
    sudo install -m 0755 "$bin" "${INSTALL_DIR}/bwenv"
  fi

  echo "Installed bwenv to ${INSTALL_DIR}/bwenv"
  case ":${PATH}:" in
    *:"${INSTALL_DIR}":*) ;;
    *)
      echo >&2
      echo "Add this directory to your PATH, e.g. for bash/zsh:" >&2
      echo "  export PATH=\"${INSTALL_DIR}:\$PATH\"" >&2
      ;;
  esac
}

main "$@"
