{
  inputs = {
    # nixpkgs.url = "github:nixos/nixpkgs/nixos-24.05";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.flake-utils.follows = "flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
  }:
    (
      flake-utils.lib.eachDefaultSystem (
        system: let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [rust-overlay.overlays.default];
          };
          packages = [
            # file://$(rustc --print sysroot)/share/doc/rust/html
            (pkgs.rust-bin.nightly."2024-06-28".default.override {
              extensions = [
                "rust-src"
                "rust-analyzer"
              ];
            })

            # testing
            pkgs.cargo-nextest
            pkgs.cargo-flamegraph
            pkgs.mold
            pkgs.llvmPackages.bintools

            # for std lib doc
            pkgs.rustup

            # wizard for perf Cargo.toml
            pkgs.cargo-wizard

            pkgs.hyperfine
            # hyperfine --show-output --warmup 5 --min-runs 10 "./target/release/filestat --path /home/marts"

            # mail
            pkgs.pkg-config
            pkgs.openssl
          ];
        in {
          devShells = rec {
            default = dev;
            dev = pkgs.mkShell {
              buildInputs = packages;
              shellHook = "export RUST_LOG=trace";
            };
          };
          packages = {
            default = pkgs.rustPlatform.buildRustPackage {
              pname = "systemd-failed";
              version = "0.0.1";
              src = ./.;
              cargoBuildFlags = "--release";

              cargoLock = {
                lockFile = ./Cargo.lock;
              };

              nativeBuildInputs = [pkgs.pkg-config pkgs.msmtp];
              buildInputs = [pkgs.openssl];

              runtimeInputs = [pkgs.msmtp];
            };
          };
        }
      )
    )
    // {
      nixosModules = rec {
        default = failed;
        failed = {
          config,
          lib,
          ...
        }: let
          cfg = config.services.systemd-failed;
        in {
          options.services.systemd-failed = {
            enable = lib.mkEnableOption "Systemd-failed";
          };

          config = lib.mkIf cfg.enable {
            systemd = {
              timers.systemd-failed = {
                wantedBy = ["timers.target"];
                timerConfig = {
                  OnBootSec = "2m";
                  OnUnitActiveSec = "2m";
                  Unit = "systemd-failed.service";
                };
              };

              services.systemd-failed = {
                description = "Notify when a systemd service failed";
                after = ["network.target"];
                wantedBy = ["multi-user.target"];
                serviceConfig = {
                  Type = "oneshot";
                  ExecStart = "${self.packages.x86_64-linux.default}/bin/systemd-failed";
                };
              };
            };
          };
        };
      };
    };
}
