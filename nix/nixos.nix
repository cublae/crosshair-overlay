# NixOS module. Installs the binaries and the desktop entry system-wide;
# per-user crosshairs belong in the home-manager module, since the daemon is a
# session process with its own config and socket.
self:
{ config, lib, pkgs, ... }:

let
  cfg = config.programs.crosshair-overlay;
in
{
  options.programs.crosshair-overlay = {
    enable = lib.mkEnableOption "the crosshair overlay";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.crosshair-overlay;
      defaultText = lib.literalExpression "crosshair-overlay.packages.\${system}.crosshair-overlay";
      description = "Package providing crosshair, crosshair-daemon and crosshair-gui.";
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];
  };
}
