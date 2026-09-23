# crosshair-overlay

Click-through crosshair overlay for Wayland compositors that speak
`wlr-layer-shell` (niri, Hyprland, Sway, river, …). Rust + GTK4.

Inspired by [Crosshair X](https://store.steampowered.com/app/1366800/Crosshair_X/),
built for Linux.

## How it works

The overlay is a layer-shell surface on the `overlay` layer anchored to all four
edges of one output, with an **empty input region** — so it sits above
fullscreen windows and every click, scroll and pointer motion passes straight
through to whatever is underneath.

Three binaries, one config:

| binary | job |
| --- | --- |
| `crosshair-daemon` | holds the overlay surface, owns the config, serves the control socket |
| `crosshair` | CLI — what you bind to keys |
| `crosshair-gui` | GTK settings window with live preview |

The daemon is the only writer of the config file, so the GUI, the CLI and your
text editor can't fight over it.

## Install

### Flake

```nix
{
  inputs.crosshair-overlay.url = "github:cublae/crosshair-overlay";
}
```

Home-manager — this is the one you want, the daemon is a session process:

```nix
imports = [ inputs.crosshair-overlay.homeManagerModules.default ];

programs.crosshair-overlay = {
  enable = true;
  autostart = true;          # systemd user service; off by default

  # Optional. Omit it entirely to keep the config writable and edit
  # crosshairs in the GUI.
  settings = {
    active = "valorant";
    monitor = "DP-5";
    visible = true;
    profiles.valorant = {
      shape = "cross";
      color = "#00ffff";
      opacity = 1.0;
      thickness = 3.0;
      length = 6.0;
      gap = 3.0;
      outline = { enabled = true; width = 1.0; color = "#000000"; };
      dot = { enabled = true; size = 1.5; };
    };
  };
};
```

Setting `settings` makes `config.toml` a read-only symlink into the store. The
daemon detects that and stops writing to it: `crosshair toggle`, profile
switches and GUI edits still take effect immediately, they are simply not
persisted, and each `home-manager switch` reloads the file live. Leave
`settings` unset if you would rather click crosshairs together in the GUI and
keep the result.

NixOS, for a system-wide install of the binaries and the desktop entry:

```nix
imports = [ inputs.crosshair-overlay.nixosModules.default ];
programs.crosshair-overlay.enable = true;
```

There is also `overlays.default`, which adds `pkgs.crosshair-overlay`.

## Build

```sh
nix develop          # rust toolchain + gtk4 + gtk4-layer-shell
cargo build --release
```

Or straight from the flake:

```sh
nix build
./result/bin/crosshair start
```

## Use

```sh
crosshair start              # start the daemon
crosshair gui                # settings window, live preview
crosshair toggle             # show / hide  ← bind this
crosshair profile sniper     # switch profile
crosshair profile --next     # cycle profiles
crosshair preset             # list built-in presets
crosshair preset valorant    # copy a preset into the config and use it
crosshair monitor DP-5       # pick the output
crosshair monitor --auto     # let the compositor choose
crosshair status
crosshair edit               # $EDITOR on the config; reloaded on save
crosshair stop
```

### Hotkeys

Wayland has no global key grab, so the compositor owns your hotkeys and the CLI
is the interface. In `~/.config/niri/config.kdl`:

```kdl
binds {
    Mod+Shift+X       { spawn "crosshair" "toggle"; }
    Mod+Shift+C       { spawn "crosshair" "profile" "--next"; }
}

spawn-at-startup "crosshair-daemon"
```

Sway/Hyprland equivalents are the same one-liners with their own `bindsym` /
`bind` syntax.

## Config

`~/.config/crosshair-overlay/config.toml`, watched for changes — save the file
and the overlay updates immediately. `CROSSHAIR_CONFIG` and `CROSSHAIR_SOCKET`
override the paths; `crosshair paths` prints what the current build uses.

```toml
active = "valorant"
monitor = "DP-5"      # omit for automatic
visible = true

[profiles.valorant]
shape = "cross"       # cross | t | x | circle | square | dot
color = "#00ffff"
opacity = 1.0
thickness = 3.0
length = 6.0          # arm length; radius for circle, half-size for square
gap = 3.0
rotation = 0.0
offset_x = 0.0        # pixels from the centre of the output
offset_y = 0.0

[profiles.valorant.outline]
enabled = true
width = 1.0
color = "#000000"

[profiles.valorant.dot]
enabled = true
size = 1.5
# color = "#ff0000"   # omitted: inherits the crosshair colour
```

`offset_x` / `offset_y` have no GUI control — they exist for games whose
viewport centre isn't the screen centre.

## A word on online games

This is an external overlay: it draws on your compositor, never touches the game
process, injects nothing and reads no game memory. That is why it works at all
on Wayland — and it is also why no anti-cheat can tell it apart from a sticker
on your monitor.

That said, **a crosshair overlay is against the rules of a number of competitive
games**, and some anti-cheats (VAC, EAC, BattlEye, Vanguard) treat overlay
software unevenly regardless of how it's implemented. Vanguard in particular
operates at a level where "external" means very little. Check the rules of the
game you're playing before you use this in a ranked match — the consequences
land on your account, not on this repo.

## Limitations

- Wayland only, and only compositors with `wlr-layer-shell`. GNOME's Mutter does
  not implement it, so this will not run there.
- One output at a time.
- No per-game profile switching — that would need compositor-specific window
  tracking.
- No tray icon; the CLI and the GUI are the whole interface.

## Licence

MIT.
