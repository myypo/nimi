# This package seems to be used as the testbed for development of Modular Services in `nixpkgs`
{
  nimi,
  ghostunnel,
}:
nimi.mkNimiBin {
  services."ghostunnel-plain-old" = {
    imports = [ ghostunnel.services.default ];
    ghostunnel = {
      listen = "0.0.0.0:443";
      cert = "/root/service-cert.pem";
      key = "/root/service-key.pem";
      disableAuthentication = true;
      target = "backend:80";
      unsafeTarget = true;
    };
  };
  services."ghostunnel-client-cert" = {
    imports = [ ghostunnel.services.default ];
    ghostunnel = {
      listen = "0.0.0.0:1443";
      cert = "/root/service-cert.pem";
      key = "/root/service-key.pem";
      cacert = "/root/ca.pem";
      target = "backend:80";
      allowCN = [ "client" ];
      unsafeTarget = true;
    };
  };
  settings.restart.mode = "up-to-count";
}
