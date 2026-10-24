/// Every component's CSS, in the order it's inlined: later files override
/// earlier ones.
pub const STYLES: [&str; 2] = [include_str!("frame.css"), include_str!("apps/apps.css")];
