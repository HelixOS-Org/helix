# =============================================================================
# Helix OS - Nix Flake Configuration
# =============================================================================
# Reproducible development environment with Nix
# https://nixos.wiki/wiki/Flakes
# =============================================================================

{
  description = "Helix OS - Modular Operating System Framework";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, crane }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Rust toolchain with custom targets
        rustToolchain = pkgs.rust-bin.nightly."2025-01-15".default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
            "rustfmt"
            "clippy"
            "llvm-tools-preview"
          ];
          targets = [
            "x86_64-unknown-none"
            "aarch64-unknown-none"
            "riscv64gc-unknown-none-elf"
          ];
        };

        # Crane for Rust builds
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Common build inputs
        buildInputs = with pkgs; [
          # Rust
          rustToolchain

          # Build tools
          gnumake
          just

          # QEMU
          qemu

          # Bootloader
          grub2
          xorriso
          mtools

          # Debugging
          gdb

          # Assembly
          nasm

          # Utilities
          binutils
          file
          xxd

          # Development
          cargo-watch
          cargo-audit
          cargo-outdated
          cargo-expand
          cargo-deny
        ];

        # Native build inputs (for build scripts)
        nativeBuildInputs = with pkgs; [
          pkg-config
          lld
        ];

      in {
        # Development shell
        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;

          shellHook = ''
            echo "🧬 Helix OS Development Environment"
            echo "===================================="
            echo ""
            echo "Rust version: $(rustc --version)"
            echo "Cargo version: $(cargo --version)"
            echo "QEMU version: $(qemu-system-x86_64 --version | head -1)"
            echo ""
            echo "Available commands:"
            echo "  make build    - Build the kernel"
            echo "  make run      - Run in QEMU"
            echo "  make test     - Run tests"
            echo "  just --list   - Show all tasks"
            echo ""

            export RUST_BACKTRACE=1
            export CARGO_HOME="$PWD/.cargo-home"
            export RUSTFLAGS="-C linker=rust-lld"
          '';

          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        };

        # Packages
        packages = {
          default = craneLib.buildPackage {
            src = craneLib.cleanCargoSource (craneLib.path ./.);

            cargoExtraArgs = "--target x86_64-unknown-none -p helix-minimal-os";

            buildInputs = buildInputs;
            nativeBuildInputs = nativeBuildInputs;

            # Don't run tests during build
            doCheck = false;
          };

          # ISO image
          iso = pkgs.stdenv.mkDerivation {
            name = "helix-iso";
            src = ./.;

            buildInputs = buildInputs;
            nativeBuildInputs = nativeBuildInputs;

            buildPhase = ''
              ./scripts/build.sh --iso
            '';

            installPhase = ''
              mkdir -p $out
              cp build/output/*.iso $out/
            '';
          };
        };

        # Apps
        apps = {
          default = flake-utils.lib.mkApp {
            drv = pkgs.writeShellScriptBin "helix-run" ''
              ${pkgs.qemu}/bin/qemu-system-x86_64 \
                -kernel ${self.packages.${system}.default}/bin/helix-minimal-os \
                -m 512M \
                -serial stdio \
                -no-reboot
            '';
          };
        };

        # Checks (for CI)
        checks = {
          fmt = craneLib.cargoFmt {
            src = craneLib.cleanCargoSource (craneLib.path ./.);
          };

          clippy = craneLib.cargoClippy {
            src = craneLib.cleanCargoSource (craneLib.path ./.);
            cargoClippyExtraArgs = "--all-targets -- -D warnings";
          };
        };
      }
    );
}
