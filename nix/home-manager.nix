# Home-manager module. Used as `inputs.crosshair-overlay.homeManagerModules.default`.
self:
{ config, lib, pkgs, ... }:

let
  cfg = config.programs.crosshair-overlay;
  tomlFormat = pkgs.formats.toml { };
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

    settings = lib.mkOption {
      type = tomlFormat.type;
      default = { };
      example = lib.literalExpression ''
        {
          active = "valorant";
          monitor = "DP-5";
          visible = true;
          profiles.valorant = {
            shape = "cross";
            color = "#00ffff";
            thickness = 3.0;
            length = 6.0;
            gap = 3.0;
            outline = { enabled = true; width = 1.0; color = "#000000"; };
            dot = { enabled = true; size = 1.5; };
          };
        }
      '';
      description = ''
        Contents of {file}`$XDG_CONFIG_HOME/crosshair-overlay/config.toml`.

        Leave empty to let the daemon manage the file itself — that is the right
        choice if you want to edit crosshairs in the GUI and keep the result.

        When set, the file becomes a read-only symlink into the Nix store. The
        daemon notices this: `crosshair toggle`, profile switches and GUI edits
        still take effect immediately, they just are not written back, and every
        `home-manager switch` reloads the file live.
      '';
    };

    autostart = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = ''
        Start the daemon with the graphical session through a systemd user
        service. Leave this off to start it from your compositor config
        instead, e.g. niri's {option}`spawn-at-startup`.

        The service needs the session environment, so the compositor must have
        imported WAYLAND_DISPLAY into the systemd user manager before
        {file}`graphical-session.target` is reached.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion =
          !(cfg.settings ? active)
          || builtins.hasAttr cfg.settings.active (cfg.settings.profiles or { });
        message = ''
          programs.crosshair-overlay.settings.active is "${cfg.settings.active or ""}",
          but no such profile exists in settings.profiles.
        '';
      }
    ];

    home.packages = [ cfg.package ];

    xdg.configFile."crosshair-overlay/config.toml" = lib.mkIf (cfg.settings != { }) {
      source = tomlFormat.generate "crosshair-overlay-config.toml" cfg.settings;
    };

    systemd.user.services.crosshair-overlay = lib.mkIf cfg.autostart {
      Unit = {
        Description = "Crosshair overlay";
        PartOf = [ "graphical-session.target" ];
        After = [ "graphical-session.target" ];
      };
      Service = {
        Type = "simple";
        ExecStart = "${cfg.package}/bin/crosshair-daemon";
        ExecReload = "${cfg.package}/bin/crosshair reload";
        Restart = "on-failure";
        RestartSec = 2;
        Slice = "session.slice";
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };
  };
}
