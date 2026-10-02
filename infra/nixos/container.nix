{ pkgs, ... }:
{
  boot.isContainer = true;
  system.stateVersion = "24.11";
  networking.hostName = "rom-dev";
  time.timeZone = "Europe/Warsaw";
  environment.systemPackages = with pkgs; [
    git rustc cargo rustfmt clippy gcc pkg-config openssl
    nodejs_22 ripgrep curl cacert
  ];
  environment.variables = {
    CARGO_BUILD_JOBS = "2";
    OPENSPEC_TELEMETRY = "0";
  };
  users.users.rom = {
    isNormalUser = true;
    home = "/home/rom";
    createHome = true;
  };
  systemd.tmpfiles.rules = [ "d /workspace 0755 rom users -" ];
}
