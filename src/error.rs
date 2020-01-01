use std::{fmt::Display, process::exit};

use log::*;

pub trait UnwrapLoggable<T> {
    fn unwrap_log(self) -> T;
    fn map_string_error(self) -> Result<T, String>;
}

impl<T, E> UnwrapLoggable<T> for Result<T, E>
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

    fn map_string_error(self) -> Result<T, String> {
        self.map_err(|err| format!("{}", err))
    }
}

//trait ExpectLoggable<T> {
//    fn expect_log(self, msg: &str) -> T;
//}
//
//impl<T> ExpectLoggable<T> for Option<T> {
//    fn expect_log(self, msg: &str) -> T {
//        match self {
//            Some(t) => t,
//            None => {
//                error!("{}", msg);
//                exit(1);
//            }
//        }
//    }
//}

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
