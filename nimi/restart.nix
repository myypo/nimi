{ lib, ... }:
let
  inherit (lib) mkOption types;
in
{
  options.settings.restart = mkOption {
    description = ''
      Decides how nimi should be restarted
    '';
    type = types.submodule {
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
    default = { };
  };
}
