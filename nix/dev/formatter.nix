{ inputs, ... }:
{
  imports = [ inputs.treefmt-nix.flakeModule ];

  perSystem.treefmt = {
    projectRootFile = "flake.nix";

    settings.global.excludes = [
      "*.envrc"
      "*.envrc."
    ];

    programs = {
      deadnix.enable = true;
      nixfmt.enable = true;
      statix.enable = true;

      mdformat.enable = true;

      rustfmt.enable = true;

      shellcheck.enable = true;
      shfmt.enable = true;

      toml-sort.enable = true;
    };
  };
}
