pub const NIX: &str = match option_env!("NIX") {
    Some(nix) => nix,
    None => "nix",
};

pub const NURL: &str = match option_env!("NURL") {
    Some(nurl) => nurl,
    None => "nurl",
};
