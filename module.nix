{ pkgs, lib, config, ... }:

with lib;

let
  cfg = config.services.pullomatic;

  repoFormat = pkgs.formats.yaml { };

  repos = pkgs.linkFarm "pullomatic.config" (mapAttrsToList
    (name: val: {
      inherit name;
      path = repoFormat.generate "${name}.yaml" val;
    })
    cfg.repos);

  pullomatic = pkgs.callPackage ./package.nix { };

in
{
  options.services.pullomatic = {
    enable = mkEnableOption "pullomatic";

    repos = mkOption {
      type = types.attrsOf (repoFormat.type);
      default = { };
    };
  };

  config = mkIf cfg.enable {
    systemd.services.pullomatic = {
      description = "Pullomatic";
      requires = [ "network-online.target" ];
      after = [ "network-online.target" ];
      wantedBy = [ "multi-user.target" ];

      restartTriggers = [ repos ];

      serviceConfig = {
        Type = "simple";
        Restart = "always";
        ExecStart = "${pullomatic}/bin/pullomatic --config '${repos}'";
      };
    };
  };
}

