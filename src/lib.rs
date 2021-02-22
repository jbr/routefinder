#![forbid(unsafe_code, future_incompatible)]
#![deny(
    missing_debug_implementations,
    nonstandard_style,
    missing_copy_implementations,
    unused_qualifications
)]

//! # routefinder
//!
//! check out [`Router`] for a good starting place
//!
//!
//! ```rust
//! use routefinder::Router;
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
//!
//! assert_eq!(
//!     router.best_match("/hey/mars").unwrap().captures().get("world"),
//!     Some("mars")
//! );
//!
//! assert_eq!(router.matches("/hello").len(), 3);
//!
//! assert_eq!(router.matches("/").len(), 1);
//! # Ok(()) }
//! ```

mod captures;
mod r#match;
mod route;
mod router;
mod segment;

pub use captures::Captures;
pub use r#match::Match;
pub use route::{Route, RouteSpec};
pub use router::Router;
pub use segment::Segment;
