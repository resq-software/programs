#!/usr/bin/env bash

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

# agent-sync.sh - Synchronize AGENTS.md and CLAUDE.md files.
#
# Usage:
#   ./agent-sync.sh              # Sync AGENTS.md -> CLAUDE.md (AGENTS.md is source)
#   ./agent-sync.sh --check      # Verify sync status without making changes.
#   ./agent-sync.sh --reverse    # Sync CLAUDE.md -> AGENTS.md (CLAUDE.md is source).
#
# Exit codes:
#   0  Sync succeeded or files are in sync (--check mode).
#   1  Mismatch found or sync failed.

set -euo pipefail

# SCRIPT_DIR stores the absolute path to the directory containing this script.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# CHECK_MODE determines if the script should only verify sync status.
CHECK_MODE=false
# REVERSE_MODE determines if CLAUDE.md should be treated as the source of truth.
REVERSE_MODE=false

for arg in "$@"; do
    case "$arg" in
        --check) CHECK_MODE=true ;;
        --reverse) REVERSE_MODE=true ;;
        *) echo "Unknown option: $arg"; exit 1 ;;
    esac
done

if [[ "$REVERSE_MODE" == "true" ]]; then
    SOURCE_FILE="CLAUDE.md"
    TARGET_FILE="AGENTS.md"
    DIRECTION="CLAUDE.md -> AGENTS.md"
else
    SOURCE_FILE="AGENTS.md"
    TARGET_FILE="CLAUDE.md"
    DIRECTION="AGENTS.md -> CLAUDE.md"
fi

echo "Syncing $DIRECTION..."

FOUND=0
SYNCED=0
MISMATCH=0

# Use git ls-files to find all SOURCE_FILE instances, including untracked ones.
while IFS= read -r -d '' source_path; do
    dir=$(dirname "$source_path")
    target_path="$dir/$TARGET_FILE"
    FOUND=$((FOUND + 1))

    if [[ "$CHECK_MODE" == "true" ]]; then
        if [[ ! -f "$target_path" ]]; then
            echo "MISSING: $target_path"
            MISMATCH=$((MISMATCH + 1))
        elif ! diff -q "$source_path" "$target_path" > /dev/null 2>&1; then
            echo "MISMATCH: $target_path differs from $source_path"
            MISMATCH=$((MISMATCH + 1))
        fi
    else
        # Only copy if files differ to avoid unnecessary writes.
        if [[ ! -f "$target_path" ]] || ! diff -q "$source_path" "$target_path" > /dev/null 2>&1; then
            cp "$source_path" "$target_path"
            SYNCED=$((SYNCED + 1))
            echo "  $source_path -> $target_path"
        fi
    fi
done < <(git ls-files -z -c -o --exclude-standard "*/$SOURCE_FILE" "$SOURCE_FILE")

if [[ "$CHECK_MODE" == "true" ]]; then
    echo ""
    echo "Check complete: $FOUND $SOURCE_FILE files found"
    if [[ $MISMATCH -gt 0 ]]; then
        echo "Issues found: $MISMATCH"
        exit 1
    else
        echo "All $TARGET_FILE files are in sync"
        exit 0
    fi
else
    echo ""
    if [[ $SYNCED -gt 0 ]]; then
        echo "Sync complete: $SYNCED files updated"
    else
        echo "Sync complete: All files were already up to date"
    fi
fi
