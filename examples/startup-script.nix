{
  nimi,
  writeShellApplication,
  lib,
}:
nimi.mkNimiBin {
  settings.startup.runOnStartup = lib.getExe (writeShellApplication {
    name = "example-startup-script";
    text = ''
      echo "hello world"
    '';
  });
}
