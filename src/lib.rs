#![forbid(unsafe_code, future_incompatible)]
#![deny(
    missing_debug_implementations,
    nonstandard_style,
    missing_copy_implementations,
    unused_qualifications
)]

mod captures;
mod matches;
mod route;
mod router;
mod segment;

pub use captures::Captures;
pub use matches::{Match, Matches};
pub use route::{Route, RouteSpec};
pub use router::Router;
pub use segment::Segment;
