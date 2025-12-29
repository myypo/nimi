{ self, ... }:
{
  perSystem =
    { inputs', ... }:
    {
      packages.docs = inputs'.ndg.packages.ndg-builder.override {
        title = "Nimi";

        rawModules = [
          self.modules.nimi.default
        ];
      };
    };
}
