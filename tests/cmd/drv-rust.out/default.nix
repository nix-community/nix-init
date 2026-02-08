{
  lib,
  stdenv,
  fetchFromGitHub,
  cargo,
  meson,
  ninja,
  pkg-config,
  rustPlatform,
  rustc,
  wrapGAppsHook4,
  cairo,
  dbus,
  gdk-pixbuf,
  glib,
  gtk4,
  libevdev,
  libpulseaudio,
  pango,
  udev,
}:

stdenv.mkDerivation (finalAttrs: {
  pname = "sway-osd";
  version = "0.3.0";

  src = fetchFromGitHub {
    owner = "ErikReider";
    repo = "SwayOSD";
    tag = "v${finalAttrs.version}";
    hash = "sha256-DRJ4D+QcgkVZmlfbj2HEIUHnYldzIuSDcpsOAOuoaL0=";
  };

  cargoDeps = rustPlatform.fetchCargoVendor {
    inherit (finalAttrs) pname version src;
    hash = "sha256-t0IZvO7Wbx6A7v/sRZOSOLj0O/1m7vOBjZSd99TAutI=";
  };

  nativeBuildInputs = [
    cargo
    meson
    ninja
    pkg-config
    rustPlatform.cargoSetupHook
    rustc
    wrapGAppsHook4
  ];

  buildInputs = [
    cairo
    dbus
    gdk-pixbuf
    glib
    gtk4
    libevdev
    libpulseaudio
    pango
    udev
  ];

  meta = {
    description = "[..]";
    homepage = "https://github.com/ErikReider/SwayOSD";
    license = lib.licenses.gpl3Only;
    maintainers = with lib.maintainers; [ alice ];
    mainProgram = "sway-osd";
    platforms = lib.platforms.all;
  };
})
