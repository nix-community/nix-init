use semver::Version;

use crate::{inputs::AllInputs, lang::go::GoPackage, macros::input_macros};

pub(super) fn load_go_dependency(inputs: &mut AllInputs, pkg: GoPackage<'_>) {
    input_macros!(inputs);

    match pkg.name {
        "github.com/diamondburned/gotk4-adwaita/pkg" => build!("libadwaita"),
        "github.com/diamondburned/gotk4/pkg" => {
            native_build!("pkg-config", "wrapGAppsHook4");
            build!("gtk4");
        }
        "github.com/dlasky/gotk3-layershell" => {
            native_build!("pkg-config");
            build!("gtk-layer-shell");
        }
        "github.com/getlantern/systray" => {
            native_build!("pkg-config");
            build!("libayatana-appindicator"; linux);
            framework!("Cocoa", "WebKit");
        }
        "github.com/google/gopacket" => build!("libpcap"),
        "github.com/gotk3/gotk3" => {
            native_build!("pkg-config", "wrapGAppsHook");
            build!("gtk3");
        }
        "github.com/itchio/ox" => framework!("Cocoa"),
        "github.com/itchio/screw" => framework!("Cocoa"),
        "github.com/shirou/gopsutil" => {
            if pkg.version.get().is_some_and(|version| {
                version < Version::new(3, 21, 3)
                    && (version.major != 2 || version < Version::new(2, 21, 11))
            }) {
                environ!("CGO_CFLAGS", r#""-Wno-undef-prefix""#);
            }
        }
        "golang.design/x/clipboard" => {
            build!("xorg.libX11"; linux);
            framework!("Cocoa");
        }
        _ => {}
    }
}
