{
  lib,
  nixosOptionsDoc,
  mdbook,
  stdenvNoCC,
  nixdoc,
  pkgs,
}:
let
  moduleEval = lib.evalModules {
    modules = [
      ./modules/nimi.nix
    ];
    class = "nimi";
    specialArgs = { inherit pkgs; };
  };

  moduleOptsDoc = nixosOptionsDoc {
    inherit (moduleEval) options;
  };
in
stdenvNoCC.mkDerivation {
  name = "options-doc-html";
  src = ../.;

  nativeBuildInputs = [
    mdbook
    nixdoc
  ];

  dontBuild = true;
  installPhase = ''
    mkdir -p "$out/share/nimi/docs"

    ln -sf "${moduleOptsDoc.optionsCommonMark}" docs/options.md

    nixdoc \
      --file nix/lib.nix \
      --category "" \
      --description "Nimi library functions" \
      --prefix "nimi" \
      --anchor-prefix "nimi" \
      > docs/functions.md

    mdbook build --dest-dir "$out/share/nimi/docs"
  '';
}
