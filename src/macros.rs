/// define a set af macros to work with inputs
macro_rules! input_macros {
    // workaround to escape `$`
    // so it can be used with nested macro_rules
    ($inputs:ident) => {
        input_macros!(($) $inputs);
    };

    (($_:tt) $inputs:ident) => {
        macro_rules! input {
            ($key:ident: $_($input:expr),+) => {
                input!($key: $_($input),+; always)
            };
            ($key:ident: $_($input:expr),+; $sys:ident) => {{
                $_(
                    $inputs.$key.$sys.insert($input.into());
                )+
            }};
        }

        macro_rules! environ {
            ($name:expr, $value:expr) => {
                environ!($name, $value;);
            };
            ($name:expr, $value:expr; $_($tt:tt)*) => {
                $inputs.env.insert(
                    $name.into(),
                    ($value.into(), vec![$_($tt)*]),
                );
            };
        }

        // native build inputs
        macro_rules! native_build {
            ($_($tt:tt)+) => {
                input!(native_build_inputs: $_($tt)+)
            };
        }

        // build inputs
        macro_rules! build {
            ($_($tt:tt)+) => {
                input!(build_inputs: $_($tt)+)
            };
        }

        // apple frameworks
        macro_rules! framework {
            ($_($input:literal),+) => {
                build!($_(concat!("darwin.apple_sdk.frameworks.", $input)),+; darwin)
            };
        }

        // gstreamer libraries
        macro_rules! gst {
            ($_($input:literal),+) => {
                build!($_(concat!("gst_all_1.", $input)),+)
            };
        }
    };
}

pub(crate) use input_macros;
