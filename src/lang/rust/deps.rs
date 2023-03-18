use cargo::core::{PackageId, Resolve};
use semver::Version;

use crate::inputs::AllInputs;

pub(super) fn load_rust_depenendency(inputs: &mut AllInputs, resolve: &Resolve, pkg: PackageId) {
    macro_rules! input {
        ($key:ident: $($input:expr),+) => {
            input!($key: $($input),+; always)
        };
        ($key:ident: $($input:expr),+; $sys:ident) => {{
            $(
                inputs.$key.$sys.insert($input.into());
            )+
        }};
    }

    // native build inputs
    macro_rules! native_build {
        ($($tt:tt)+) => {
            input!(native_build_inputs: $($tt)+)
        };
    }

    // build inputs
    macro_rules! build {
        ($($tt:tt)+) => {
            input!(build_inputs: $($tt)+)
        };
    }

    // apple frameworks
    macro_rules! framework {
        ($($input:literal),+) => {
            build!($(concat!("darwin.apple_sdk.frameworks.", $input)),+; darwin)
        };
    }

    // gstreamer libraries
    macro_rules! gst {
        ($($input:literal),+) => {
            build!($(concat!("gst_all_1.", $input)),+)
        };
    }

    match &*pkg.name() {
        "alsa-sys" => build!("alsa-lib"),
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
        "core-foundation-sys" => framework!("CoreFoundation"),
        "core-graphics-types" => framework!("CoreGraphics"),
        "core-text" => framework!("CoreText"),
        "coreaudio-sys" => framework!("CoreAudio"),
        "curl-sys" => build!("curl"),
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
        "gtk-sys" => build!("gtk3"),
        "gtk4-sys" => build!("gtk4"),
        "io-kit-sys" => framework!("IOKit"),
        "io-surface" => build!("IOSurface"),
        "libadwaita-sys" => build!("libadwaita"),
        "libdbus-sys" => build!("dbus"),
        "libgit2-sys" => build!("libgit2"),
        "libgpg-error-sys" => {
            native_build!("libgpg-error");
            build!("libgpg-error")
        }
        "libpulse-sys" => build!("libpulseaudio"),
        "libsecret-sys" => build!("libsecret"),
        "libshumate-sys" => build!("libshumate"),
        "libsodium-sys" => build!("libsodium"),
        "libsodium-sys-stable" => build!("libsodium"),
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
        "lzma-sys" => build!("xz"),
        "metal" => framework!("Metal"),
        "ncurses" => build!("ncurses"),
        "nettle-sys" => build!("nettle"),
        "onig_sys" => {
            build!("oniguruma");
            inputs.env.insert("RUSTONIG_SYSTEM_LIBONIG", "true");
        }
        "openssl-sys" => {
            build!("openssl");
            framework!("Security");
        }
        "pam-sys" => build!("pam"),
        "pango-sys" => build!("pango"),
        "pipewire-sys" => build!("pipewire"),
        "pkg-config" => native_build!("pkg-config"),
        "pq-sys" => build!("postgresql"),
        "prost-build" => native_build!("protobuf"),
        "rdkafka-sys" => build!("rdkafka"),
        "ring" => framework!("Security"),
        "security-framework-sys" => framework!("Security"),
        "servo-fontconfig-sys" => build!("fontconfig"),
        "smithay-client-toolkit" => build!("libxkbcommon"),
        "soup3-sys" => build!("libsoup"),
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
        "xcb" => build!("xorg.libxcb"; linux),
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
                inputs.env.insert("ZSTD_SYS_USE_PKG_CONFIG", "true");
            }
        }
        _ => {}
    }
}
