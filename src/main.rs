use std::{env, fs};

mod decode;
mod segment;
mod ui;

pub(crate) use decode::DisplaySet;

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let mut args = env::args();
    let file = args.nth(1).unwrap();
    let bytes = fs::read(file)?;
    let frames = decode::parse_frames(&bytes).map_err(|err| eyre::eyre!("{err:?}"))?;
    let num_frames = frames.len();

    println!("processed {num_frames} frames");

    iced::application(
        move || ui::SupViewer::new(frames.clone()),
        ui::SupViewer::update,
        ui::SupViewer::view,
    )
    .centered()
    .run()?;

    Ok(())
}
