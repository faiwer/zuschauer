//! A series as a line graph: an SVG line inside HTML axes and labels, so
//! the text doesn't stretch with the SVG.
//!
//! - `scale`: round value ticks around the data
//! - `time`: round time ticks within the data
//! - `render`: the markup

mod render;
mod scale;
mod time;

pub use render::render_graph;
