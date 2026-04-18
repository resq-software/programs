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

{
  description = "ResQ Programs — Solana Anchor on-chain programs (airspace, delivery)";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-24.11";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    { self, nixpkgs, flake-utils, rust-overlay, ... }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      mkDevShell = pkgs: system:
        let
          # Anchor 0.30.1 requires a pinned nightly; stable works for most program development
          rustToolchain = pkgs.rust-bin.stable.latest.default.override {
            extensions = [ "rust-src" "rust-analyzer" "rustfmt" "clippy" ];
            targets = [ "bpfel-unknown-none" ];  # BPF target for Solana programs
          };

          devPackages = with pkgs;
            builtins.filter (p: p != null) [
              # Rust (required for Anchor + Solana programs)
              rustToolchain

              # Node / Bun (Anchor client-side tests and scripts)
              nodejs_22
              bun

              # Build deps
              pkg-config
              git
              jq
              osv-scanner
              curl

              # Linux build deps for solana-sdk crates
              (if stdenv.isLinux then openssl else null)
              (if stdenv.isLinux then libudev-zero else null)
            ] ++ lib.optionals stdenv.isDarwin [
              darwin.apple_sdk.frameworks.Security
              darwin.apple_sdk.frameworks.CoreFoundation
              darwin.apple_sdk.frameworks.SystemConfiguration
            ];

          shellHook = ''
            echo "--- ResQ Programs Dev Environment (${system}) ---"

            version_check() {
              local cmd="$1" name="$2"
              if command -v "$cmd" >/dev/null 2>&1; then
                echo "$name: $("$cmd" --version 2>/dev/null | head -n1 | xargs)"
              else
                echo "$name: NOT FOUND"
              fi
            }

            version_check rustc  "Rust"
            version_check cargo  "Cargo"
            version_check node   "Node"
            version_check bun    "Bun"
            version_check solana "Solana CLI"
            version_check anchor "Anchor"

            if ! command -v solana >/dev/null 2>&1; then
              echo ""
              echo "⚠  Solana CLI not found. Install via:"
              echo "   sh -c \"\$(curl -sSfL https://release.anza.xyz/stable/install)\""
            fi

            if ! command -v anchor >/dev/null 2>&1; then
              echo ""
              echo "⚠  Anchor CLI not found. Install via:"
              echo "   cargo install --git https://github.com/coral-xyz/anchor avm --locked"
              echo "   avm install 0.30.1 && avm use 0.30.1"
            fi

            echo ""
            echo "Build:  anchor build"
            echo "Test:   bash ./scripts/test.sh"
            echo "Deploy: anchor deploy --provider.cluster devnet"
            echo "-------------------------------------------------"

            export CARGO_HOME="$PWD/.cargo"
            export PATH="$CARGO_HOME/bin:$PATH"
          '';
        in
        {
          default = pkgs.mkShell {
            packages = devPackages;
            inherit shellHook;
          };
        };
    in
    flake-utils.lib.eachSystem supportedSystems (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
          config.allowUnfree = true;
        };
      in
      {
        formatter = pkgs.alejandra or pkgs.nixpkgs-fmt;
        devShells = mkDevShell pkgs system;
      }
    );
}
