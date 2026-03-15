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

# ── Stage 1: Rust + Solana toolchain ─────────────────────────────────────────
# Build time is significant (~10 min cold); use BuildKit cache mounts in CI.
FROM rust:1-slim AS toolchain

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    pkg-config \
    libssl-dev \
    libudev-dev \
    clang \
    cmake \
    git \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install Bun (used by Anchor for JS client tests)
RUN curl -fsSL https://bun.sh/install | bash
ENV PATH="/root/.bun/bin:$PATH"

# Install Solana CLI
ARG SOLANA_VERSION=2.1.0
RUN sh -c "$(curl -sSfL https://release.anza.xyz/v${SOLANA_VERSION}/install)"
ENV PATH="/root/.local/share/solana/install/active_release/bin:$PATH"

# Add BPF target for Solana programs
RUN rustup target add bpfel-unknown-none

# Install Anchor via AVM — pinned to match workspace version
ARG ANCHOR_VERSION=0.30.1
RUN cargo install --git https://github.com/coral-xyz/anchor avm --locked \
    && avm install ${ANCHOR_VERSION} \
    && avm use ${ANCHOR_VERSION}

# ── Stage 2: build programs ───────────────────────────────────────────────────
FROM toolchain AS builder
WORKDIR /app

# Install JS dependencies first for layer caching
COPY package.json bun.lock* ./
RUN bun install 2>/dev/null || bun install --no-frozen-lockfile

COPY . .
RUN anchor build

# ── Stage 3: test runner ──────────────────────────────────────────────────────
FROM builder AS test
RUN anchor test --skip-deploy

# Default target: produce built artifacts (*.so + IDL JSON)
FROM builder AS artifacts
# Built programs are at: target/deploy/*.so
# IDL files are at:      target/idl/*.json
CMD ["ls", "-lh", "target/deploy/", "target/idl/"]
