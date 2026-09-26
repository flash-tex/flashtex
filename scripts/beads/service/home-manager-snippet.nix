# Proposed home-manager module fragment for GoKubar/nixos-config (NOT applied by
# the tooling agent; the owner adds it). Equivalent to flashtex-beads-sync.service.
# Replace the repo path with the machine's main flashtex clone.
{ config, pkgs, ... }:
let repo = "${config.home.homeDirectory}/code/flashtex"; in
{
  systemd.user.services.flashtex-beads-sync = {
    Unit = { Description = "FlashTeX Beads ledger sync loop"; After = [ "network-online.target" ]; };
    Service = {
      WorkingDirectory = repo;
      Environment = [
        "FLASHTEX_BEADS_STATE=${config.home.homeDirectory}/.local/state/flashtex-beads"
        "PATH=${config.home.homeDirectory}/.local/share/flashtex-beads/bin:${pkgs.git}/bin:${pkgs.openssh}/bin:${pkgs.coreutils}/bin"
      ];
      ExecStart = "${pkgs.python3}/bin/python3 ${repo}/scripts/beads/sync-loop --repo ${repo} --max-backoff 60";
      Restart = "on-failure";
      RestartSec = 30;
      RestartPreventExitStatus = "2 75";
    };
    Install.WantedBy = [ "default.target" ];
  };
}
# And in the NixOS system configuration (lets the pinned, checksum-verified bd run
# without the loader shim):
#   programs.nix-ld.enable = true;
