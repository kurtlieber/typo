//! The markdown core: document model, block and inline parsers, serializer,
//! HTML renderer, edits, history and queries. Pure Rust, no Qt, so all of it
//! is unit-tested with `cargo test`.

pub mod edit;
pub mod from_html;
pub mod history;
pub mod html;
pub mod imgtag;
pub mod inline;
pub mod model;
pub mod pair;
pub mod parse;
pub mod query;
pub mod serialize;
pub mod smart;
pub mod table;

pub use model::*;
pub use parse::{parse, parse_with};
pub use serialize::serialize;

#[cfg(test)]
mod tests;
