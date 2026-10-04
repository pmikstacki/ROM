# Prepared module. Import only after the exact release artifact passes acceptance.
{ releaseDirectory, studioArgs, publicHost ? "bdziam.home.arpa" }:
{ lib, ... }:
{
  containers.rom-studio = {
    autoStart = true;
    privateNetwork = false;
    bindMounts = {
      "/opt/rom-studio" = { hostPath = releaseDirectory; isReadOnly = true; };
      "/var/lib/rom-studio" = { hostPath = "/var/lib/rom-studio-preview/data"; isReadOnly = false; };
      "/run/rom-studio-config" = { hostPath = "/var/lib/rom-studio-preview/config"; isReadOnly = true; };
      "/run/rom-studio-private" = { hostPath = "/var/lib/rom-studio-preview/private"; isReadOnly = true; };
    };
    config = import ./service-container.nix { inherit studioArgs; };
  };
  # These routes reuse the existing VPN listener. No firewall, WG, DNS or SSH changes.
  services.caddy.virtualHosts."https://${publicHost}".extraConfig = lib.mkBefore ''
    redir /rom-studio /rom-studio/ 308
    handle /rom-studio/* {
      reverse_proxy 127.0.0.1:44173
    }
    handle /rom-studio-provider/* {
      reverse_proxy 127.0.0.1:44174
    }
  '';
}
