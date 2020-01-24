use std::{fmt::Display, process::exit};

use hyper;
use log::*;
use thiserror::Error;

pub trait UnwrapLoggable<T> {
    fn unwrap_log(self) -> T;
    fn map_string_error(self) -> Result<T, String>;
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("hyper error")]
    HyperError {
        #[from]
        source: hyper::Error,
    },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl<T, E> UnwrapLoggable<T> for std::result::Result<T, E>
where
    E: Display,
{
    fn unwrap_log(self) -> T {
        match self {
            Ok(t) => t,
            Err(e) => {
                error!("{}", e);
                exit(1);
            }
        }
    }

    fn map_string_error(self) -> std::result::Result<T, String> {
        self.map_err(|err| format!("{}", err))
    }
}

#[macro_export]
macro_rules! expect_log {
    ($x:expr, $($fmt:expr),+ ) => {
        match $x {
            Some(t) => t,
            None => {
                error!($($fmt),+);
                exit(1);
            }
        }
    }
}
