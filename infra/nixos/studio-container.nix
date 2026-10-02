{ ... }:
{
  boot.isContainer = true;
  system.stateVersion = "24.11";
  networking.hostName = "rom-studio";

  # The host gateway owns network exposure and TLS. No public container port.
  services.nginx = {
    enable = true;
    virtualHosts.studio = {
      listen = [ { addr = "127.0.0.1"; port = 44173; } ];
      root = "/srv/studio";
      locations."/".extraConfig = ''
        autoindex off;
        add_header Cache-Control "no-store" always;
      '';
    };
  };
}
