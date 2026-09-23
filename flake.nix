{
  description = "Crosshair overlay for Wayland — Rust + GTK4 + wlr-layer-shell";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = f:
        nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (pkgs: rec {
        crosshair-overlay = pkgs.callPackage ./nix/package.nix { };
        default = crosshair-overlay;
      });

      overlays.default = final: _prev: {
        crosshair-overlay = final.callPackage ./nix/package.nix { };
      };

      nixosModules.default = import ./nix/nixos.nix self;
      homeManagerModules.default = import ./nix/home-manager.nix self;

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            pkg-config
            cargo
            rustc
            rustfmt
            clippy
            rust-analyzer
          ];
          buildInputs = with pkgs; [ glib gtk4 gtk4-layer-shell wayland ];
        };
      });

      formatter = forAllSystems (pkgs: pkgs.nixpkgs-fmt);
    };
}
