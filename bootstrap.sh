#!/usr/bin/env sh
# Copyright 2026 ResQ
# SPDX-License-Identifier: Apache-2.0
#
# Canonical onboarding — delegates to resq-software/dev.
# See https://github.com/resq-software/dev for the full installer.
set -eu
export REPO=programs
exec sh -c "$(curl -fsSL https://raw.githubusercontent.com/resq-software/dev/main/install.sh)"
