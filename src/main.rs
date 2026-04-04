use std::{env, fs};

mod decode;
mod ocr;
mod segment;
mod ui;

pub(crate) use decode::DisplaySet;

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let mut args = env::args();
    let file = args.nth(1).unwrap();
    let bytes = fs::read(file)?;
    let frames = decode::parse_frames(&bytes).map_err(|err| eyre::eyre!("{err:?}"))?;
    let mut ocr_engine: Box<dyn ocr::OcrEngine> = match ocr::TesseractOcrEngine::new("eng") {
        Ok(engine) => Box::new(engine),
        Err(err) => Box::new(ocr::NoopOcrEngine::new(err.to_string())),
    };
    let ocr_frames = ocr::recognize_frames(&mut *ocr_engine, &frames);
    let num_frames = frames.len();

    println!("processed {num_frames} frames");

    iced::application(
        move || ui::SupViewer::new(frames.clone(), ocr_frames.clone()),
        ui::SupViewer::update,
        ui::SupViewer::view,
    )
    .centered()
    .run()?;

    Ok(())
}
