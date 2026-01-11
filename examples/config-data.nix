{
  nimi,
  lib,
  http-server,
}:
nimi.mkNimiBin {
  services."has-config-data" = {
    process.argv = [
      (lib.getExe http-server)
    ];
    configData."my-config-file" = {
      enable = true;
      text = ''
        hello world
      '';
      path = "my-config-file.txt";
    };
  };
  settings.restart.mode = "always";
}
