{ lib, ... }:
{
  containers.rom-studio = {
    autoStart = true;
    privateNetwork = false;
    bindMounts."/srv/studio" = {
      hostPath = "/var/lib/rom-studio-preview/site";
      isReadOnly = true;
    };
    config = import ./studio-container.nix;
  };

  # Reuse the host's existing VPN-only HTTPS listeners and certificates.
  # This module adds no firewall rules, DNS records or SSH configuration.
  services.caddy.virtualHosts = lib.genAttrs
    [ "https://10.66.0.2" "https://bdziam.home.arpa" ]
    (_: {
      extraConfig = lib.mkBefore ''
        redir /rom-studio /rom-studio/ 308
        handle_path /rom-studio/* {
          reverse_proxy 127.0.0.1:44173
        }
      '';
    });
}
