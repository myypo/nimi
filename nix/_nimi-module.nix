{ nixpkgs, ... }:
{
  lib,
  pkgs,
  ...
}:
let
  restartType = types.submodule {
    options = {
      mode = mkOption {
        description = ''
          The restart mode to use for nimi
        '';
        default = "always";
        type = types.enum [
          "never"
          "up-to-count"
          "always"
        ];
      };
      time = mkOption {
        description = ''
          Amount of time (ms) to wait between process restarts
        '';
        type = types.ints.positive;
        default = 10;
      };
      count = mkOption {
        description = ''
          If `mode` == `up-to-count`, the maximum amount of times to restart
          before exiting
        '';
        type = types.ints.positive;
        default = 5;
      };
    };
  };
  startupType = types.submodule {
    options = {
      runOnStartup = mkOption {
        description = ''
          Binary to run on startup
        '';
        type = types.nullOr types.pathInStore;
        default = null;
      };
    };
  };

  settingsType = types.submodule {
    options = {
      restart = mkOption {
        description = ''
          Decides how nimi should be restarted
        '';
        type = restartType;
        default = { };
      };
      startup = mkOption {
        description = ''
          Decides how nimi should start up
        '';
        type = startupType;
        default = { };
      };
    };
  };

  inherit (lib) mkOption types;
in
{
  options.services = mkOption {
    description = ''
      Services to run inside the nimi runtime
    '';
    type = types.attrsOf (
      types.submoduleWith {
        class = "service";
        modules = [
          (lib.modules.importApply "${nixpkgs}/nixos/modules/system/service/portable/service.nix" {
            inherit pkgs;
          })
        ];
      }
    );
    default = { };
    visible = "shallow";
  };

  options.settings = mkOption {
    description = ''
      Settings for the nimi instance itself
    '';
    type = settingsType;
    default = { };
  };
}
