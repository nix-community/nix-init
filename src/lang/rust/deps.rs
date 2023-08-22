use cargo::core::{PackageId, Resolve};
use semver::Version;

use crate::{inputs::AllInputs, macros::input_macros};

pub(super) fn load_rust_dependency(inputs: &mut AllInputs, resolve: &Resolve, pkg: PackageId) {
    input_macros!(inputs);

    match &*pkg.name() {
        "alsa-sys" => build!("alsa-lib"; linux),
        "arboard" => framework!("AppKit"),
        "ash" => build!("vulkan-loader"),
        "atk-sys" => build!("atk"),
        "bindgen" => native_build!("rustPlatform.bindgenHook"),
        "bzip2-sys" => build!("bzip2"),
        "cairo-sys-rs" => build!("cairo"),
        "clipboard" => framework!("AppKit"),
        "cocoa" => framework!("AppKit"),
        "cocoa-foundation" => framework!("Foundation"),
        "copypasta" => framework!("AppKit"),
        "core-graphics-types" => framework!("CoreGraphics"),
        "core-text" => framework!("CoreText"),
        "coreaudio-sys" => framework!("CoreAudio"),
        "curl-sys" => {
            native_build!("curl");
            build!("curl");
        }
        "evdev-sys" => build!("evdev-sys"),
        "expat-sys" => build!("expat"),
        "freetype-sys" => build!("freetype"),
        "fsevent-sys" => framework!("CoreFoundation", "CoreServices"),
        "gdk-pixbuf-sys" => build!("gdk-pixbuf"),
        "gdk-sys" => build!("gtk3"),
        "gdk4-sys" => build!("gtk4"),
        "glib-sys" => build!("glib"),
        "gpgme-sys" => {
            native_build!("gpgme");
            build!("gpgme")
        }
        "gsk4-sys" => build!("gtk4"),
        "gstreamer-allocators-sys" => gst!("gst-plugins-base"),
        "gstreamer-app-sys" => gst!("gst-plugins-base"),
        "gstreamer-audio-sys" => gst!("gst-plugins-base"),
        "gstreamer-base-sys" => gst!("gstreamer"),
        "gstreamer-check-sys" => gst!("gstreamer"),
        "gstreamer-controller-sys" => gst!("gstreamer"),
        "gstreamer-editing-services-sys" => gst!("gst-editing-services"),
        "gstreamer-gl-sys" => gst!("gst-plugins-base"),
        "gstreamer-mpegts-sys" => gst!("gst-plugins-bad"),
        "gstreamer-net-sys" => gst!("gstreamer"),
        "gstreamer-pbutils-sys" => gst!("gst-plugins-base"),
        "gstreamer-play-sys" => gst!("gst-plugins-bad"),
        "gstreamer-player-sys" => gst!("gst-plugins-bad"),
        "gstreamer-rtp-sys" => gst!("gst-plugins-base"),
        "gstreamer-rtsp-server-sys" => gst!("gst-rtsp-server"),
        "gstreamer-rtsp-sys" => gst!("gst-plugins-base"),
        "gstreamer-sdp-sys" => gst!("gst-plugins-base"),
        "gstreamer-sys" => gst!("gstreamer"),
        "gstreamer-tag-sys" => gst!("gst-plugins-base"),
        "gstreamer-validate-sys" => gst!("gst-devtools"),
        "gstreamer-video-sys" => gst!("gst-plugins-base"),
        "gstreamer-webrtc-sys" => gst!("gst-plugins-bad"),
        "gtk-sys" => {
            native_build!("wrapGAppsHook");
            build!("gtk3");
        }
        "gtk4-sys" => {
            native_build!("wrapGAppsHook4");
            build!("gtk4");
        }
        "io-kit-sys" => framework!("IOKit"),
        "io-surface" => build!("IOSurface"),
        "libadwaita-sys" => build!("libadwaita"),
        "libdbus-sys" => build!("dbus"),
        "libgit2-sys" => build!("libgit2"),
        "libgpg-error-sys" => {
            native_build!("libgpg-error");
            build!("libgpg-error")
        }
        "libhandy-sys" => {
            if pkg.version() < &Version::new(0, 7, 0) {
                build!("libhandy_0");
            } else {
                build!("libhandy");
            }
        }
        "libpanel-sys" => build!("libpanel"),
        "libpulse-sys" => build!("libpulseaudio"),
        "librocksdb-sys" => {
            environ!("ROCKSDB_INCLUDE_DIR", r#""${rocksdb}/include""#; "rocksdb".into());
            environ!("ROCKSDB_LIB_DIR", r#""${rocksdb}/lib""#);
            if resolve.features(pkg).iter().any(|feat| feat == "io-uring") {
                build!("liburing"; linux);
            }
        }
        "libsecret-sys" => build!("libsecret"),
        "libshumate-sys" => build!("libshumate"),
        "libsodium-sys" | "libsodium-sys-stable" => {
            build!("libsodium");
            if resolve
                .features(pkg)
                .iter()
                .all(|feat| feat != "use-pkg-config")
            {
                environ!("SODIUM_USE_PKG_CONFIG", "true");
            }
        }
        "libspa-sys" => build!("pipewire"),
        "libsqlite3-sys" => build!("sqlite"),
        "libudev-sys" => build!("udev"),
        "libusb1-sys" => build!("libusb"),
        "libxml" => build!("libxml2"),
        "libz-sys" => {
            let mut ng = false;
            let mut stock = false;
            for feat in resolve.features(pkg) {
                match &**feat {
                    "static" => return,
                    "stock-zlib" => stock = true,
                    "zlib-ng" => ng = true,
                    _ => {}
                }
            }
            if stock || !ng {
                build!("zlib");
            }
        }
        "llvm-sys" => {
            build!("libxml2", "ncurses", "zlib");
            let major = pkg.version().major;
            let llvm_pkgs = format!("llvmPackages_{}", major / 10);
            environ!(
                format!("LLVM_SYS_{major}_PREFIX"),
                format!("{llvm_pkgs}.llvm.dev");
                llvm_pkgs,
            );
        }
        "locate-dwarf" => framework!("CoreServices"),
        "lzma-sys" => build!("xz"),
        "metal" => framework!("Metal"),
        "ncurses" => build!("ncurses"),
        "nettle-sys" => build!("nettle"),
        "onig_sys" => {
            build!("oniguruma");
            environ!("RUSTONIG_SYSTEM_LIBONIG", "true");
        }
        "openssl-sys" => {
            build!("openssl");
            framework!("Security");
            if resolve.features(pkg).iter().any(|feat| feat == "vendored") {
                environ!("OPENSSL_NO_VENDOR", "true");
            }
        }
        "pam-sys" => build!("pam"),
        "pango-sys" => build!("pango"),
        "pipewire-sys" => build!("pipewire"),
        "pkg-config" => native_build!("pkg-config"),
        "poppler-sys-rs" => build!("poppler"),
        "pq-sys" => build!("postgresql"),
        "prost-build" => native_build!("protobuf"),
        "rdkafka-sys" => build!("rdkafka"),
        "readkey" => framework!("AppKit"),
        "readmouse" => framework!("AppKit"),
        "ring" => framework!("Security"),
        "security-framework-sys" => framework!("Security"),
        "servo-fontconfig-sys" => build!("fontconfig"),
        "smithay-client-toolkit" => build!("libxkbcommon"),
        "soup-sys" => build!("libsoup"),
        "soup2-sys" => build!("libsoup"),
        "soup3-sys" => build!("libsoup_3"),
        "sourceview5-sys" => build!("gtksourceview5"),
        "spirv-tools-sys" => build!("spirv-tools"),
        "sys-locale" => framework!("CoreFoundation"),
        "sysinfo" => framework!("IOKit"),
        "wayland-sys" => build!("wayland"; linux),
        "webkit2gtk-sys" => build!("webkitgtk"),
        "webkit2gtk-webextension-sys" => build!("webkitgtk"),
        "webkit2gtk5-sys" => build!("webkitgtk_5_0"),
        "webkit2gtk5-webextension-sys" => build!("webkitgtk_5_0"),
        "wgpu-hal" => framework!("QuartzCore"),
        "whoami" => framework!("CoreFoundation", "SystemConfiguration"),
        "wholesym" => framework!("CoreServices"),
        "wireplumber" => build!("wireplumber"),
        "x11" => {
            for feat in resolve.features(pkg) {
                // https://github.com/AltF02/x11-rs/blob/fced94ef6eb5935c892079a46812806f7b7a9237/x11/build.rs#L14
                let dep = match &**feat {
                    "glx" => "libGL",
                    "xlib" => "xorg.libX11",
                    "xlib_xcb" => "xorg.libX11",
                    "xcursor" => "xorg.libXcursor",
                    "dpms" => "xorg.libXext",
                    "xfixes" => "xorg.libXfixes",
                    "xft" => "xorg.libXft",
                    "xinput" => "xorg.libX1",
                    "xinerama" => "xorg.libXinerama",
                    "xmu" => "xorg.libXmu",
                    "xrandr" => "xorg.libXrandr",
                    "xrender" => "xorg.libXrender",
                    "xpresent" => "xorg.libXpresent",
                    "xss" => "xorg.libXScrnSaver",
                    "xt" => "xorg.libXt",
                    "xtst" => "xorg.libXtst",
                    "xf86vmode" => "xorg.libXxf86vm",
                    _ => continue,
                };
                build!(dep; linux);
            }
        }
        "xcb" => {
            build!("xorg.libxcb"; linux);
            if pkg.version() < &Version::new(0, 10, 0) {
                native_build!("python3"; linux);
            }
        }
        "xkbcommon" => build!("libxkbcommon"),
        "xkbcommon-sys" => build!("libxkbcommon"),
        "yeslogic-fontconfig-sys" => build!("fontconfig"),
        "zmq-sys" => build!("zeromq"),
        "zstd-sys" => {
            if resolve
                .features(pkg)
                .iter()
                .any(|feat| feat == "pkg-config")
            {
                build!("zstd");
            } else if pkg.version() >= &Version::new(2, 0, 5) {
                build!("zstd");
                environ!("ZSTD_SYS_USE_PKG_CONFIG", "true");
            }
        }
        _ => {}
    }
}
