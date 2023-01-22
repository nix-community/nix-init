use tracing::warn;

use std::fmt::Display;

pub trait ResultExt {
    type Output;

    fn ok_warn(self) -> Option<Self::Output>;
}

impl<T, E: Display> ResultExt for Result<T, E> {
    type Output = T;

    fn ok_warn(self) -> Option<Self::Output> {
        self.map_err(|e| warn!("{e}")).ok()
    }
}
