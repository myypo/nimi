{ nixpkgs, ... }:
{
  lib,
  pkgs,
  ...
}:
let
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
}
