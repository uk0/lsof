#!/usr/bin/env bash
set -euo pipefail

REPO="uk0/lsof"
BINARY="loof"
INSTALL_DIR="/usr/local/bin"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[INFO]${NC} $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*"; exit 1; }

# Detect OS and architecture
detect_platform() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)  os="linux" ;;
        Darwin) os="macos" ;;
        *)      error "Unsupported OS: $os" ;;
    esac

    case "$arch" in
        x86_64|amd64)   arch="amd64" ;;
        aarch64|arm64)  arch="arm64" ;;
        *)              error "Unsupported architecture: $arch" ;;
    esac

    # Map to artifact name
    case "${os}-${arch}" in
        linux-amd64)  echo "loof-linux-amd64" ;;
        macos-arm64)  echo "loof-macos-arm64" ;;
        macos-amd64)  warn "No native x86_64 macOS build; trying arm64 (Rosetta 2)"; echo "loof-macos-arm64" ;;
        *)            error "No prebuilt binary for ${os}-${arch}" ;;
    esac
}

# Get latest release tag
get_latest_version() {
    local url="https://api.github.com/repos/${REPO}/releases/latest"
    local json version

    if command -v curl &>/dev/null; then
        json=$(curl -fsSL "$url")
    elif command -v wget &>/dev/null; then
        json=$(wget -qO- "$url")
    else
        error "Neither curl nor wget found. Please install one of them."
    fi

    # Extract tag_name value — portable across macOS/Linux sed
    version=$(echo "$json" | grep -o '"tag_name" *: *"[^"]*"' | head -1 | grep -o '"v[^"]*"' | tr -d '"')

    [ -z "$version" ] && error "Failed to fetch latest version. Check https://github.com/${REPO}/releases"
    echo "$version"
}

# Download and install
install() {
    local platform version download_url tmp_dir archive

    platform="$(detect_platform)"
    info "Detected platform: ${platform}"

    version="$(get_latest_version)"
    info "Latest version: ${version}"

    archive="${platform}.tar.gz"
    download_url="https://github.com/${REPO}/releases/download/${version}/${archive}"

    tmp_dir="$(mktemp -d)"
    trap 'rm -rf "$tmp_dir"' EXIT

    info "Downloading ${download_url} ..."
    if command -v curl &>/dev/null; then
        curl -fSL "$download_url" -o "${tmp_dir}/${archive}"
    else
        wget -q "$download_url" -O "${tmp_dir}/${archive}"
    fi

    info "Extracting ..."
    tar xzf "${tmp_dir}/${archive}" -C "$tmp_dir"

    info "Installing to ${INSTALL_DIR}/${BINARY} ..."
    if [ -w "$INSTALL_DIR" ]; then
        mv "${tmp_dir}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
    else
        sudo mv "${tmp_dir}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
    fi
    chmod +x "${INSTALL_DIR}/${BINARY}"

    info "Verifying installation ..."
    if command -v "$BINARY" &>/dev/null; then
        echo ""
        info "loof installed successfully!"
        "${INSTALL_DIR}/${BINARY}" --version
        echo ""
        info "Run 'loof -h' for usage or 'loof -I' for interactive TUI mode."
    else
        warn "Installed to ${INSTALL_DIR}/${BINARY} but not found in PATH."
        warn "Add ${INSTALL_DIR} to your PATH or move the binary manually."
    fi
}

install
