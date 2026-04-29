mod app;
mod model;
mod registry;
mod tui;
mod workspace;

use anyhow::Result;

use crate::app::App;

fn main() -> Result<()> {
    let mut app = App::load()?;
    tui::run(&mut app)
}
