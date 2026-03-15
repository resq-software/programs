#!/bin/bash

# Copyright 2026 ResQ
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

#   bash 4+
#   Standard POSIX tools (curl, grep, awk, etc.)
#
# Environment:
#   OS_TYPE    Operating system type (linux, macos, windows).
#   ARCH_TYPE  System architecture (amd64, arm64, arm).
#   LOG_LEVEL  Logging verbosity (debug, info, warn, error).
#   YES        If set to 1, auto-confirm all prompts.
#
# Exit codes:
#   Functions return 0 on success, non-zero on failure.
#
# Example:
#   source "./tools/scripts/lib/shell-utils.sh"
#   log_info "Starting build..."
#   install_package "build-essential"

#######################################
# Global configuration
#######################################

# COLOR_RED is the ANSI escape code for red text.
readonly COLOR_RED='\033[0;31m'
# COLOR_GREEN is the ANSI escape code for green text.
readonly COLOR_GREEN='\033[0;32m'
# COLOR_YELLOW is the ANSI escape code for yellow text.
readonly COLOR_YELLOW='\033[1;33m'
# COLOR_BLUE is the ANSI escape code for blue text.
readonly COLOR_BLUE='\033[0;34m'
# COLOR_MAGENTA is the ANSI escape code for magenta text.
export COLOR_MAGENTA='\033[0;35m'
# COLOR_CYAN is the ANSI escape code for cyan text.
export COLOR_CYAN='\033[0;36m'
# COLOR_NC is the ANSI escape code to reset terminal color.
readonly COLOR_NC='\033[0m'

# Detects the operating system.
#
# Outputs:
#   Writes the OS name (linux, macos, windows, or unknown) to stdout.
#
# Returns:
#   0 always.
detect_os() {
    case "$(uname -s)" in
        Linux*)                 echo "linux";;
        Darwin*)                echo "macos";;
        CYGWIN*|MINGW*|MSYS*)   echo "windows";;
        *)                      echo "unknown";;
    esac
}

# Detects the system architecture.
#
# Outputs:
#   Writes the architecture name (amd64, arm64, arm, or unknown) to stdout.
#
# Returns:
#   0 always.
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)   echo "amd64";;
        arm64|aarch64)  echo "arm64";;
        armv7l)         echo "arm";;
        *)              echo "unknown";;
    esac
}

# OS_TYPE stores the detected operating system.
OS_TYPE="${OS_TYPE:-$(detect_os)}"
# ARCH_TYPE stores the detected system architecture.
ARCH_TYPE="${ARCH_TYPE:-$(detect_arch)}"

export OS_TYPE ARCH_TYPE

# Internal logging helper.
#
# Args:
#   $1 - Color escape code.
#   $2 - Log level label.
#   $* - Message to log.
#
# Outputs:
#   Writes the formatted message to stderr.
_log_message() {
    local color="$1"
    local level="$2"
    shift 2
    local message="$*"
    echo -e "${color}[${level}]${COLOR_NC} ${message}" >&2
}

# Logs an info message.
#
# Args:
#   $* - Message to log.
#
# Outputs:
#   Writes the message to stderr with [INFO] label in blue.
log_info()    { _log_message "$COLOR_BLUE"    "INFO"    "$@"; }

# Logs a success message.
#
# Args:
#   $* - Message to log.
#
# Outputs:
#   Writes the message to stderr with [SUCCESS] label in green.
log_success() { _log_message "$COLOR_GREEN"   "SUCCESS" "$@"; }

# Logs a warning message.
#
# Args:
#   $* - Message to log.
#
# Outputs:
#   Writes the message to stderr with [WARNING] label in yellow.
log_warning() { _log_message "$COLOR_YELLOW"  "WARNING" "$@"; }

# Logs an error message.
#
# Args:
#   $* - Message to log.
#
# Outputs:
#   Writes the message to stderr with [ERROR] label in red.
log_error()   { _log_message "$COLOR_RED"     "ERROR"   "$@"; }

# Gets high-resolution timestamp.
#
# Outputs:
#   Writes seconds since epoch with decimal precision to stdout.
#
# Returns:
#   0 on success, non-zero on failure.
get_high_res_time() {
    if [[ "$OS_TYPE" == "macos" ]]; then
        # macOS date doesn't support %N, use python as fallback
        python3 -c 'import time; print(time.time())' 2>/dev/null || date +%s
    else
        date +%s.%N 2>/dev/null || date +%s
    fi
}

# Checks if a command exists in PATH.
#
# Args:
#   $1 - Command name to check.
#
# Returns:
#   0 if the command exists.
#   1 if the command does not exist.
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Detects package manager for current OS.
#
# Outputs:
#   Writes the package manager name to stdout.
#
# Returns:
#   0 always.
get_package_manager() {
    case "$OS_TYPE" in
        linux)
            if command_exists apt-get; then echo "apt"
            elif command_exists dnf; then echo "dnf"
            elif command_exists yum; then echo "yum"
            elif command_exists pacman; then echo "pacman"
            elif command_exists zypper; then echo "zypper"
            elif command_exists apk; then echo "apk"
            else echo "unknown"; fi
            ;;
        macos)
            if command_exists brew; then echo "brew"
            else echo "none"; fi
            ;;
        windows)
            if command_exists scoop; then echo "scoop"
            elif command_exists winget; then echo "winget"
            elif command_exists choco; then echo "choco"
            else echo "none"; fi
            ;;
        *)
            echo "unknown"
            ;;
    esac
}

# Installs a package using the appropriate package manager.
#
# Args:
#   $1 - Package name to install.
#
# Returns:
#   0 on success.
#   1 on failure or if package manager is unknown.
#
# Requirements:
#   Sudo/root privileges for system packages.
install_package() {
    local package="$1"
    local pkg_mgr
    pkg_mgr=$(get_package_manager)

    local sudo_cmd="sudo"
    if [[ "$EUID" -eq 0 ]]; then sudo_cmd=""; fi

    # Non-interactive flags for every package manager
    case "$pkg_mgr" in
        apt) $sudo_cmd apt-get update -y && $sudo_cmd apt-get install -y "$package" ;;
        dnf) $sudo_cmd dnf install -y "$package" ;;
        yum) $sudo_cmd yum install -y "$package" ;;
        pacman) $sudo_cmd pacman -Sy --noconfirm "$package" ;;
        apk) $sudo_cmd apk add --no-cache "$package" ;;
        brew) brew install --quiet "$package" ;;
        choco) choco install -y "$package" ;;
        scoop) scoop install "$package" ;;
        winget) winget install --silent --accept-source-agreements --accept-package-agreements --id "$package" ;;
        *) return 1 ;;
    esac
}

# Specialized installer for osv-scanner to handle naming differences.
install_osv_scanner() {
    local pkg_mgr
    pkg_mgr=$(get_package_manager)

    log_info "Attempting to install osv-scanner via $pkg_mgr..."

    case "$pkg_mgr" in
        winget) install_package "Google.OSVScanner" ;;
        *) install_package "osv-scanner" ;;
    esac
}

# Installs Nix and enables flakes.
#
# Returns:
#   0 on success.
#   1 if automatic installation is not supported.
#
# Side effects:
#   Creates /etc/nix/nix.conf, enables nix-daemon.
#   Adds user to nix-users group.
#
# Requirements:
#   Arch Linux (for automatic installation) or manual installation on other distros.
install_nix() {
    if command_exists nix; then
        return 0
    fi

    log_info "Nix not found. Attempting to install Nix..."

    # 1. Try Native DISTRO installation first (Best for package management integration)
    if [[ -f /etc/arch-release ]]; then
        log_info "Arch Linux detected. Attempting native install via pacman..."
        if install_package nix; then
            local sudo_cmd="sudo"
            if [[ "$EUID" -eq 0 ]]; then sudo_cmd=""; fi

            log_info "Configuring Nix daemon..."
            $sudo_cmd mkdir -p /etc/nix
            if ! grep -q "flakes" /etc/nix/nix.conf 2>/dev/null; then
                echo "experimental-features = nix-command flakes" | $sudo_cmd tee -a /etc/nix/nix.conf >/dev/null
            fi
            $sudo_cmd systemctl enable --now nix-daemon
            if ! groups | grep -q "nix-users"; then
                $sudo_cmd usermod -aG nix-users "$USER"
            fi

            # Source profile
            [ -f "/etc/profile.d/nix.sh" ] && source /etc/profile.d/nix.sh

            if command_exists nix; then
                log_success "Native Nix installed successfully!"
                return 0
            fi
        fi
        log_warning "Native pacman install failed (often due to library conflicts). Falling back to official installer..."
    fi

    # 2. Fallback to OFFICIAL MULTI-USER INSTALLER (Most resilient method)
    log_info "Running official Nix multi-user install script..."
    if curl -L https://nixos.org/nix/install | sh -s -- --daemon --yes; then
        # Source for current shell (various possible locations)
        for profile in "/etc/profile.d/nix.sh" "$HOME/.nix-profile/etc/profile.d/nix.sh" "/nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh"; do
            if [ -f "$profile" ]; then
                log_info "Activating Nix environment from $profile..."
                # shellcheck source=/dev/null
                source "$profile"
                break
            fi
        done

        if command_exists nix; then
            log_success "Nix installed and activated via official script!"
            return 0
        fi
    fi

    log_error "All Nix installation methods failed. Please install manually: https://nixos.org/download.html"
    return 1
}

# Ensures running inside a Nix environment if available.
# Re-executes the current script inside 'nix develop' if conditions are met.
#
# Args:
#   $@ - Arguments to pass to the re-executed script.
#
# Side effects:
#   May re-execute script via exec nix develop.
ensure_nix_env() {
    # Check if we are already in a Nix shell or if Nix is missing
    if [[ -n "$IN_NIX_SHELL" ]] || [[ -n "$RESQ_NIX_RECURSION" ]] || ! command_exists nix; then
        return 0
    fi

    # Check if flake.nix exists in the current project root
    local project_root
    project_root=$(git rev-parse --show-toplevel 2>/dev/null || echo ".")
    if [[ ! -f "$project_root/flake.nix" ]]; then
        return 0
    fi

    log_info "Nix detected. Entering development environment via flake.nix..."

    # Set a recursion guard
    export RESQ_NIX_RECURSION=1

    # If $0 is a file, re-execute it. Otherwise, we're likely in an interactive
    # or sourced session where we can't easily re-exec.
    if [[ -f "$0" ]]; then
        exec nix develop "$project_root" --command "$0" "$@"
    else
        # Fallback for sourced or bash -c "..." calls
        # We don't use exec here to avoid the "cannot execute binary" error
        # and instead just let the shell continue if possible, or warn.
        if [[ "${RESQ_SILENT_NIX_WARNING:-0}" -ne 1 ]]; then
            log_warning "Could not re-execute environment automatically (sourced or subshell)."
            log_info "Please run 'nix develop' manually if tools are missing."
        fi
        return 0
    fi
}

# MD5 hash wrapper (cross-platform).
#
# Args:
#   $@ - File paths to hash.
#
# Outputs:
#   Writes MD5 hash for each file to stdout.
#
# Returns:
#   0 on success.
#   1 if no MD5 command is found.
md5sum_wrapper() {
    if command_exists md5sum; then
        md5sum "$@"
    elif command_exists md5; then
        md5 -r "$@"
    elif command_exists certutil; then
        for file in "$@"; do
            certutil -hashfile "$file" MD5 | grep -v ":" | tr -d '[:space:]'
            echo "  $file"
        done
    else
        log_error "No MD5 command found"
        return 1
    fi
}

# Prompts for user confirmation.
#
# Args:
#   $1 - Message to display.
#   $2 - Default answer (optional, 'y' or 'n').
#
# Returns:
#   0 if the user answers 'y' or 'Y'.
#   1 if the user answers 'n' or 'N'.
#
# Environment:
#   YES - If set to 1, auto-confirms without prompting.
prompt() {
    local msg="$1"
    local default="${2:-}"

    # If YES is set to 1 globally, auto-confirm
    if [[ "${YES:-0}" -eq 1 ]]; then
        log_info "$msg (auto-yes)"
        return 0
    fi

    local prompt_str="(y/n)"
    if [[ "$default" == "y" ]]; then prompt_str="([y]/n)";
    elif [[ "$default" == "n" ]]; then prompt_str="(y/[n])"; fi

    read -p "${COLOR_YELLOW}?${COLOR_NC} $msg $prompt_str " -n 1 -r
    echo
    if [[ -z "$REPLY" && -n "$default" ]]; then
        REPLY="$default"
    fi
    [[ $REPLY =~ ^[Yy]$ ]]
}

# Ensures the user has sudo privileges.
#
# Exits:
#   1 if sudo is not available and not running as root.
require_sudo() {
    if [[ $EUID -ne 0 ]]; then
        if command_exists sudo; then
            log_warning "Some operations require root. You may be prompted for your password."
        else
            log_error "This script requires root privileges or sudo."
            exit 1
        fi
    fi
}

# Gets latest release tag from GitHub API.
#
# Args:
#   $1 - Repo path (e.g. "docker/compose").
#
# Outputs:
#   Writes the latest release tag (e.g. "v2.0.0") to stdout.
#
# Returns:
#   0 on success, non-zero on failure.
get_latest_github_release() {
    local repo="$1"
    curl -s "https://api.github.com/repos/${repo}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
}

# Checks if a port is in use.
#
# Args:
#   $1 - Port number.
#
# Returns:
#   0 if the port is in use.
#   1 if the port is free.
#
# Requirements:
#   lsof, netstat, or /proc/net/tcp.
check_port_in_use() {
    local port="$1"
    if command_exists lsof; then
        lsof -i :"$port" >/dev/null 2>&1
    elif command_exists netstat; then
        netstat -tuln | grep -q ":$port "
    else
        # Fallback to /proc if available
        grep -q "$(printf ":%04X" "$port")" /proc/net/tcp 2>/dev/null
    fi
}

# Generic Docker installer.
#
# Returns:
#   0 if Docker is already installed or successfully installed.
#   1 if installation fails.
#
# Side effects:
#   Installs Docker, may require system restart or re-login.
#
# Requirements:
#   Root/sudo privileges.
install_docker() {
    if command_exists docker; then return 0; fi

    require_sudo
    log_info "Attempting to install Docker..."

    case "$OS_TYPE" in
        linux)
            if command_exists apt-get; then
                sudo apt-get update -y
                sudo apt-get install -y apt-transport-https ca-certificates curl software-properties-common lsb-release
                curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo apt-key add -
                sudo add-apt-repository "deb [arch=$(dpkg --print-architecture)] https://download.docker.com/linux/ubuntu $(ls_release -cs) stable"
                sudo apt-get update -y
                sudo apt-get install -y docker-ce docker-ce-cli containerd.io
                sudo usermod -aG docker "$USER"
            elif command_exists dnf || command_exists yum; then
                local pkg_mgr=$(get_package_manager)
                sudo $pkg_mgr install -y yum-utils
                sudo $pkg_mgr-config-manager --add-repo https://download.docker.com/linux/centos/docker-ce.repo
                sudo $pkg_mgr install -y docker-ce docker-ce-cli containerd.io
                sudo systemctl start docker
                sudo systemctl enable docker
                sudo usermod -aG docker "$USER"
            elif command_exists pacman; then
                sudo pacman -S --noconfirm docker
                sudo systemctl start docker
                sudo systemctl enable docker
                sudo usermod -aG docker "$USER"
            else
                log_error "Automatic Docker install not supported for this distribution."
                return 1
            fi
            ;;
        macos)
            if command_exists brew; then
                brew install --cask docker
            else
                log_error "Homebrew not found. Please install Docker Desktop manually."
                return 1
            fi
            ;;
        *)
            log_error "Docker installation not supported for $OS_TYPE"
            return 1
            ;;
    esac
    log_success "Docker installed successfully."
}

# Generic Bun installer.
#
# Returns:
#   0 if Bun is already installed or successfully installed.
#   1 on failure.
#
# Side effects:
#   Downloads and installs Bun to ~/.bun.
#   Exports BUN_INSTALL and PATH for the current session.
install_bun() {
    if command_exists bun; then return 0; fi

    log_info "Installing Bun..."
    case "$OS_TYPE" in
        linux|macos)
            curl -fsSL https://bun.sh/install | bash
            # Export for current session
            export BUN_INSTALL="$HOME/.bun"
            export PATH="$BUN_INSTALL/bin:$PATH"
            ;;
        windows)
            if command_exists powershell.exe; then
                powershell.exe -Command "irm bun.sh/install.ps1 | iex"
            else
                log_error "PowerShell required for Bun installation on Windows."
                return 1
            fi
            ;;
    esac
    log_success "Bun installed."
}

# Ensures auditing tools are installed.
#
# Returns:
#   0 if tools are available.
#   1 if required tools are missing and user declines installation.
#
# Side effects:
#   May install osv-scanner and/or audit-ci.
#
# Requirements:
#   osv-scanner (requires sudo on Linux).
#   audit-ci (installed via bun).
ensure_audit_tools() {
    local missing=()
    local project_root
    project_root=$(git rev-parse --show-toplevel 2>/dev/null || echo ".")

    if ! command_exists osv-scanner; then
        missing+=("osv-scanner")
    fi

    # Check for audit-ci globally or in node_modules
    if ! command_exists audit-ci && [[ ! -f "$project_root/node_modules/.bin/audit-ci" ]]; then
        missing+=("audit-ci")
    fi

    if [[ ${#missing[@]} -eq 0 ]]; then
        return 0
    fi

    # Try Nix first (Best Practice)
    if command_exists nix && [[ -f "$project_root/flake.nix" ]] && [[ -z "${IN_NIX_SHELL:-}" ]]; then
        log_info "Attempting to locate tools in Nix environment..."
        if nix eval "$project_root#devShells.$(nix eval --raw "nixpkgs#system").default.nativeBuildInputs" --json 2>/dev/null | grep -q "osv-scanner"; then
             log_info "Auditing tools found in Nix flake. Run 'nix develop' to activate."
        fi
    fi

    log_warning "Missing auditing tools: ${missing[*]}"

    # Fallback to automatic installation
    if [[ "${YES:-0}" -eq 1 ]] || prompt "Would you like to install missing auditing tools?"; then
        for tool in "${missing[@]}"; do
            case "$tool" in
                osv-scanner)
                    log_info "Attempting to install osv-scanner via system package manager..."
                    if ! install_osv_scanner; then
                        log_info "System install failed or unavailable. Falling back to Go install..."
                        if command_exists go; then
                            go install github.com/google/osv-scanner/v2/cmd/osv-scanner@latest
                            export PATH="$(go env GOPATH)/bin:$PATH"
                        else
                            log_error "Go not found. Cannot install osv-scanner."
                            return 1
                        fi
                    fi
                    ;;
                audit-ci)
                    log_info "Installing audit-ci via Bun..."
                    cd "$project_root" && bun install && cd - >/dev/null
                    ;;
            esac
        done
    else
        log_error "Auditing tools are required for pre-commit checks. Please install them manually."
        return 1
    fi
}
