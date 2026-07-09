#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::missing_errors_doc)]
#![allow(missing_docs)]
#![deny(clippy::mod_module_files)]
// I'm sorry to the stable Rust users
#![allow(incomplete_features)]
#![feature(lazy_type_alias)]
#![feature(sync_nonpoison)]
#![feature(nonpoison_mutex)]
#![feature(optimize_attribute)]

pub mod codecs;
pub mod handlers;
pub mod net;
pub mod proto;
pub mod server;
pub mod world;
