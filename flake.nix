{
  description = "pullomatic - automated git pulls";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    flake-utils.url = "github:numtide/flake-utils";

    pre-commit-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };

  outputs = { self, nixpkgs, flake-utils, pre-commit-hooks, ... }:
    (flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

      in
      {
        checks = {
          inherit (self.packages.${system}) pullomatic;

          pre-commit-check = pre-commit-hooks.lib.${system}.run {
            src = ./.;

            hooks = {
              nixpkgs-fmt.enable = true;

              clippy = {
                enable = true;
                settings.allFeatures = true;
              };

              cargo-check = {
                enable = true;
              };

              rustfmt = {
                enable = true;
              };
            };
          };
        };

        packages = {
          pullomatic = pkgs.callPackage ./package.nix { };
          default = self.packages.${system}.pullomatic;
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ self.packages.${system}.pullomatic ]
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
          #RUST_SRC_PATH = "${pkgs.rust}/lib.rs/rustlib/src/rust/library";
        };
      })
    ) // {
      nixosModules = {
        pullomatic = ./module.nix;
        default = self.nixosModules.pullomatic;
      };
    };
}
