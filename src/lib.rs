#![forbid(unsafe_code)]
#![deny(
    clippy::dbg_macro,
    missing_copy_implementations,
    rustdoc::missing_crate_level_docs,
    missing_debug_implementations,
    nonstandard_style,
    unused_qualifications
)]
#![warn(missing_docs)]

//! # Routefinder
//!
//! ```rust
//! use routefinder::{Router, Captures};
//! # pub fn main() -> Result<(), String> {
//! let mut router = Router::new();
//! router.add("/*", 1)?;
//! router.add("/hello", 2)?;
//! router.add("/:greeting", 3)?;
//! router.add("/hey/:world", 4)?;
//! router.add("/hey/earth", 5)?;
//! router.add("/:greeting/:world/*", 6)?;
//!
//! assert_eq!(*router.best_match("/hey/earth").unwrap(), 5);
//! assert_eq!(
//!     router.best_match("/hey/mars").unwrap().captures().get("world"),
//!     Some("mars")
//! );
//! assert_eq!(router.matches("/hello").len(), 3);
//! assert_eq!(router.matches("/").len(), 1);
//!
//! # Ok(()) }
//! ```
//!
//! Check out [`Router`] for a good starting place
//!

mod captures;
pub use captures::{Capture, Captures};

mod r#match;
pub use r#match::Match;

mod route;
pub use route::Route;

mod router;
pub use router::Router;

mod segment;
pub use segment::Segment;

mod reverse_match;
pub use reverse_match::ReverseMatch;

mod route_spec;
pub use route_spec::RouteSpec;
