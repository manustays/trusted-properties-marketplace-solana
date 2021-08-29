/// lib.rs -> Registering the modules

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

pub mod instruction;
pub mod processor;
pub mod state;
pub mod error;

// Error Logging
// macro_rules! loge {
// 	($msg:expr) => {{
// 		msg!("{}\n --> {}:{}:{}", format_args!($($args)*), file!(), line!(), column!());
// 	}};
// }
