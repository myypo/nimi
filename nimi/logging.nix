{ lib, ... }:
let
  inherit (lib) mkOption types;
in
{
  options.settings.logging = mkOption {
    description = ''
      Logging behavior for the nimi process manager.

      TODO
    '';
    type = types.submodule {
      options = {
        enableLogFiles = mkOption {
          description = ''
            If files for each services' logs should be written to `settings.logging.logsDir`
          '';
          type = types.bool;
          default = false;
        };
        logsDir = mkOption {
          description = ''
            Directory to (create and) write per service logs to

            Happens at runtime
          '';
          type = types.str;
          default = "nimi_logs";
        };
      };
    };
    default = { };
  };
}
