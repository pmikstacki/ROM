# Future declarative counterpart for the scoped application-only update.
{ releaseName, studioArgs, publicHost ? "bdziam.home.arpa" }:
{ lib, ... }:
{
  containers.rom-studio = {
    autoStart = true;
    privateNetwork = false;
    bindMounts."/srv/studio" = {
      hostPath = "/var/lib/rom-studio-preview/site";
      isReadOnly = true;
    };
    config = import ./existing-container.nix { inherit releaseName studioArgs; };
  };
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
