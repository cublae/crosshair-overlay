{ lib
, rustPlatform
, pkg-config
, wrapGAppsHook4
, glib
, gtk4
, gtk4-layer-shell
, wayland
}:

rustPlatform.buildRustPackage {
  pname = "crosshair-overlay";
  version = "0.1.0";

  src = lib.cleanSource ../.;
  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [ pkg-config wrapGAppsHook4 ];
  buildInputs = [ glib gtk4 gtk4-layer-shell wayland ];

  postInstall = ''
    install -Dm444 data/crosshair-overlay.desktop \
      -t $out/share/applications
    install -Dm444 data/crosshair-overlay.svg \
      $out/share/icons/hicolor/scalable/apps/crosshair-overlay.svg
  '';

  meta = {
    description = "Click-through crosshair overlay for Wayland compositors";
    mainProgram = "crosshair";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
  };
}
