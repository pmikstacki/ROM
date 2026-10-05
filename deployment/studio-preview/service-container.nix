{ studioArgs }:
{ lib, pkgs, ... }:
let
  common = {
    User = "rom-studio";
    Group = "rom-studio";
    Restart = "on-failure";
    RestartSec = 2;
    KillSignal = "SIGTERM";
    KillMode = "control-group";
    TimeoutStopSec = 45;
    NoNewPrivileges = true;
    PrivateTmp = true;
    ProtectSystem = "strict";
    ProtectHome = true;
    RestrictSUIDSGID = true;
    UMask = "0077";
    LoadCredential = "client-secret:/run/rom-studio-private/client-secret";
  };
in {
  boot.isContainer = true;
  system.stateVersion = "24.11";
  networking.hostName = "rom-studio";
  # The existing container shares the host network namespace. Never start DHCP here.
  networking.useDHCP = false;
  users.groups.rom-studio.gid = 44173;
  users.users.rom-studio = { isSystemUser = true; uid = 44173; group = "rom-studio"; };
  systemd.services.rom-studio-provider = {
    description = "ROM demonstration identity provider (not production authentication)";
    wantedBy = [ "multi-user.target" ];
    serviceConfig = common // {
      ExecStart = "${pkgs.nodejs_22}/bin/node /opt/rom-studio/provider/preview-provider.mjs /run/rom-studio-config/provider.json";
    };
  };
  systemd.services.rom-studio = {
    description = "ROM 0.0.2 protected Studio preview";
    wantedBy = [ "multi-user.target" ];
    after = [ "rom-studio-provider.service" ];
    wants = [ "rom-studio-provider.service" ];
    serviceConfig = common // {
      ExecStart = lib.escapeShellArgs ([ "/opt/rom-studio/bin/rom-demo" ] ++ studioArgs);
      ReadWritePaths = [ "/var/lib/rom-studio" ];
      WorkingDirectory = "/var/lib/rom-studio";
    };
  };
}
