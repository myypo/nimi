{ lib, ... }:
let
  inherit (lib) mkOption types;
in
{
  options.settings.startup = mkOption {
    description = ''
      Decides how nimi should start up
    '';
    type = types.submodule {
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
    default = { };
  };
}
