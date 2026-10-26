use std::io::Write;

use crate::collect::{self, Parts};
use crate::config::Config;

use super::print::render;
use super::wait::settle;

#[tokio::main(flavor = "current_thread")]
pub async fn show(config: &'static Config, parts: Parts) -> ! {
    collect::spawn(config, parts);
    settle(config, parts).await;
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(render(config, parts, terminal_width()).as_bytes());
    let _ = stdout.flush();
    // Shutting the runtime down would wait for a storage read stuck on a
    // hung network mount.
    std::process::exit(0)
}

/// `None` when stdout isn't a terminal.
fn terminal_width() -> Option<usize> {
    // SAFETY: `winsize` is plain integers, for which all-zero is valid.
    let mut size: libc::winsize = unsafe { std::mem::zeroed() };
    // SAFETY: TIOCGWINSZ writes one `winsize` into `size`.
    let status = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut size) };
    (status == 0 && size.ws_col > 0).then_some(usize::from(size.ws_col))
}
