use crate::{inputs::AllInputs, macros::input_macros};

pub(super) fn load_go_dependency(inputs: &mut AllInputs, pkg: &str) {
    input_macros!(inputs);

    match pkg {
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
        "github.com/go-gl/glfw" => {
            build!(
                "xorg.libX11",
                "xorg.libXcursor",
                "xorg.libXi",
                "xorg.libXinerama",
                "xorg.libXrandr";
                linux
            );
            framework!("Cocoa", "Kernel");
        }
        "github.com/google/gopacket" => build!("libpcap"),
        "github.com/gotk3/gotk3" => {
            native_build!("pkg-config", "wrapGAppsHook");
            build!("gtk3");
        }
        "github.com/itchio/ox" => framework!("Cocoa"),
        "github.com/itchio/screw" => framework!("Cocoa"),
        "golang.design/x/clipboard" => {
            build!("xorg.libX11"; linux);
            framework!("Cocoa");
        }
        _ => {}
    }
}
