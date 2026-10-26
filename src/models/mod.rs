pub mod config;
pub mod geometry;
pub mod monitor;
pub mod resolution;

pub use config::DisplayConfiguration;
pub use geometry::{LogicalRect, Point, Transform};
pub use monitor::{Monitor, RawHyprlandMonitor};
pub use resolution::{Mode, Resolution};
