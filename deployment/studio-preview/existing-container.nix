# Scoped update for the existing /srv/studio read-only bind.
# Mutable data and credentials remain in the retained container root.
{ releaseName, studioArgs }:
assert builtins.isString releaseName && builtins.stringLength releaseName <= 128
  && builtins.match "[A-Za-z0-9_-]+[A-Za-z0-9_.-]*" releaseName != null
  && releaseName != "." && releaseName != "..";
{ ... }:
{
  imports = [ (import ./service-container.nix { inherit studioArgs; }) ];
  systemd.tmpfiles.rules = [
    "d /var/lib/rom-studio 0700 rom-studio rom-studio -"
    "d /var/lib/rom-studio-config 0755 root root -"
    "d /var/lib/rom-studio-private 0700 root root -"
    "L+ /opt/rom-studio - - - - /srv/studio/releases/${releaseName}"
    "L+ /run/rom-studio-config - - - - /var/lib/rom-studio-config"
    "L+ /run/rom-studio-private - - - - /var/lib/rom-studio-private"
  ];
}
