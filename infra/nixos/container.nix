{ pkgs, ... }:
{
  # Pin the overlay (including component hashes) and stable Rust release.
  # The container OS remains on its existing nixpkgs release.
  nixpkgs.overlays = [
    (import (builtins.fetchTarball {
      url = "https://github.com/oxalica/rust-overlay/archive/368fee9beaab04ca6fe7af28db63caa9badb22fa.tar.gz";
      sha256 = "153wynqcjizxi46vh9xxf1rw5z7b59jhchma5mnxzv3iyhvwrgg6";
    }))
  ];
  boot.isContainer = true;
  system.stateVersion = "24.11";
  networking.hostName = "rom-dev";
  time.timeZone = "Europe/Warsaw";
  environment.systemPackages = with pkgs; [
    git rust-bin.stable."1.99.0".default gcc pkg-config openssl
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
