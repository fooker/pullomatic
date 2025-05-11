{
  description = "pullomatic - automated git pulls";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    crane = {
      url = "github:ipetkov/crane";
    };

    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };

    pre-commit-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay, pre-commit-hooks, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        craneLib = (crane.mkLib pkgs).overrideToolchain rust;

        pullomatic = craneLib.buildPackage {
          pname = "pullomatic";

          src = craneLib.path ./.;

          strictDeps = true;

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgs; [
            openssl
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
          ];
        };
      in
      {
        checks = {
          inherit pullomatic;

          pre-commit-check = pre-commit-hooks.lib.${system}.run {
            src = ./.;

            hooks = {
              nixpkgs-fmt.enable = true;

              clippy = {
                enable = true;
                settings.allFeatures = true;
                package = rust;
              };

              cargo-check = {
                enable = true;
                package = rust;
              };

              rustfmt = {
                enable = true;
                package = rust;
              };
            };
          };
        };

        packages = {
          inherit pullomatic;
          default = self.packages.${system}.pullomatic;
        };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};

          inputsFrom = [ pullomatic ]
            ++ self.checks.${system}.pre-commit-check.enabledPackages;

          packages = with pkgs; [
            cargo-deny
            cargo-outdated
            cargo-machete

            codespell

            nodejs
          ];

          inherit (self.checks.${system}.pre-commit-check) shellHook;

          RUST_BACKTRACE = 1;
          RUST_SRC_PATH = "${rust}/lib.rs/rustlib/src/rust/library";
        };
      });
}
