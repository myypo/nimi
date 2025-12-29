{
  flake-parts-lib,
  lib,
  ...
}:
let
  inherit (flake-parts-lib) mkPerSystemOption;
  inherit (lib) mkOption types;
in
{
  options.perSystem = mkPerSystemOption {
    options.mkContainerImage = mkOption {
      description = ''
        Create a ready to use OCI image from 
        nimi config
      '';
      type = types.functionTo types.package;
    };
  };

  config.perSystem =
    { inputs', config, ... }:
    {
      mkContainerImage =
        module:
        inputs'.nix2container.packages.nix2container.buildImage {
          name = "nimi-container";
          config = {
            entrypoint = [
              (lib.getExe (config.evalServicesConfig module))
            ];
          };
        };
    };

}
