{
  description = "Container PID 1 and process runner for Nix Modular Services";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.nix2container = {
    url = "github:nlewo/nix2container";
    inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    { nixpkgs, nix2container, ... }:
    let
      inherit (nixpkgs) lib;

      overlay = final: _prev: {
        nimi = final.callPackage ./nix/package.nix {
          inherit (nix2container.packages.${final.stdenv.hostPlatform.system}) nix2container;
        };
      };

      eachSystem =
        fn:
        lib.genAttrs lib.systems.flakeExposed (
          system:
          fn {
            inherit system;
            pkgs = nixpkgs.legacyPackages.${system}.appendOverlays [
              overlay
            ];
          }
        );
    in
    {
      packages = eachSystem (
        { pkgs, system, ... }:
        import ./default.nix {
          inherit pkgs;
          inherit (nix2container.packages.${system}) nix2container;
        }
      );
      checks = eachSystem (
        { pkgs, ... }:
        let
          checksFromDir =
            directory:
            lib.packagesFromDirectoryRecursive {
              inherit (pkgs) callPackage;
              inherit directory;
            };
        in
        (checksFromDir ./examples) // (checksFromDir ./nix/checks)
      );

      devShells = eachSystem (
        { pkgs, ... }:
        {
          default = import ./shell.nix { inherit pkgs; };
        }
      );
      formatter = eachSystem ({ pkgs, ... }: pkgs.callPackage ./nix/formatter.nix { });
      overlays.default = overlay;
    };

  nixConfig = {
    extra-substituters = [
      "https://weyl-ai.cachix.org"
    ];
    extra-trusted-public-keys = [
      "weyl-ai.cachix.org-1:cR0SpSAPw7wejZ21ep4SLojE77gp5F2os260eEWqTTw="
    ];
  };
}
