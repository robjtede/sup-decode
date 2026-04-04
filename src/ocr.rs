use std::{
    env, fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::NaiveTime;
use eyre::{Context as _, Result, bail};

use crate::DisplaySet;

const TRANSPARENT: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

#[derive(Debug, Clone)]
pub(crate) struct SubtitleRaster {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) pixels: Vec<u8>,
}

#[derive(Debug, Clone)]
pub(crate) struct OcrWord {
    pub(crate) text: String,
    pub(crate) confidence: Option<f32>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct OcrData {
    pub(crate) text: String,
    pub(crate) mean_confidence: Option<f32>,
    pub(crate) words: Vec<OcrWord>,
}

#[derive(Debug, Clone)]
pub(crate) enum OcrState {
    NotConfigured(String),
    Recognized(OcrData),
    Failed(String),
}

#[derive(Debug, Clone)]
pub(crate) struct OcrFrame {
    pub(crate) pts: NaiveTime,
    pub(crate) backend: &'static str,
    pub(crate) subtitle_size: (u32, u32),
    pub(crate) state: OcrState,
}

pub(crate) trait OcrEngine {
    fn name(&self) -> &'static str;

    fn is_configured(&self) -> bool {
        true
    }

    fn not_configured_reason(&self) -> Option<&str> {
        None
    }

    fn recognize(&mut self, raster: &SubtitleRaster) -> Result<OcrData>;
}

#[derive(Debug)]
pub(crate) struct NoopOcrEngine {
    reason: String,
}

impl Default for NoopOcrEngine {
    fn default() -> Self {
        Self {
            reason: "no OCR backend configured".to_owned(),
        }
    }
}

impl NoopOcrEngine {
    pub(crate) fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl OcrEngine for NoopOcrEngine {
    fn name(&self) -> &'static str {
        "none"
    }

    fn is_configured(&self) -> bool {
        false
    }

    fn not_configured_reason(&self) -> Option<&str> {
        Some(&self.reason)
    }

    fn recognize(&mut self, _raster: &SubtitleRaster) -> Result<OcrData> {
        Ok(OcrData::default())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TesseractOcrEngine {
    binary: PathBuf,
    language: String,
    page_segmentation_mode: u8,
}

impl TesseractOcrEngine {
    pub(crate) fn new(language: impl Into<String>) -> Result<Self> {
        let binary = PathBuf::from("tesseract");
        let version = Command::new(&binary)
            .arg("--version")
            .output()
            .with_context(|| "failed to execute `tesseract --version`")?;

        if !version.status.success() {
            bail!("`tesseract --version` exited with {}", version.status);
        }

        Ok(Self {
            binary,
            language: language.into(),
            page_segmentation_mode: 7,
        })
    }
}

impl OcrEngine for TesseractOcrEngine {
    fn name(&self) -> &'static str {
        "tesseract"
    }

    fn recognize(&mut self, raster: &SubtitleRaster) -> Result<OcrData> {
        let image_path = temp_ocr_path("png");
        let pixels = raster_to_luma(raster);

        image::save_buffer(
            &image_path,
            &pixels,
            raster.width,
            raster.height,
            image::ColorType::L8,
        )
        .with_context(|| format!("write OCR input image to {}", image_path.display()))?;

        let output = Command::new(&self.binary)
            .arg(&image_path)
            .arg("stdout")
            .arg("-l")
            .arg(&self.language)
            .arg("--psm")
            .arg(self.page_segmentation_mode.to_string())
            .arg("tsv")
            .arg("quiet")
            .output()
            .with_context(|| "run tesseract OCR")?;

        let _ = fs::remove_file(&image_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("tesseract failed: {}", stderr.trim());
        }

        let tsv = String::from_utf8(output.stdout).context("decode tesseract TSV output")?;
        parse_tesseract_tsv(&tsv)
    }
}

pub(crate) fn recognize_frames(engine: &mut dyn OcrEngine, frames: &[DisplaySet]) -> Vec<OcrFrame> {
    frames
        .iter()
        .map(|frame| {
            let raster = rasterize_subtitle(frame);
            let subtitle_size = raster
                .as_ref()
                .map(|raster| (raster.width, raster.height))
                .unwrap_or((0, 0));

            let state = if !engine.is_configured() {
                OcrState::NotConfigured(
                    engine
                        .not_configured_reason()
                        .unwrap_or("OCR backend unavailable")
                        .to_owned(),
                )
            } else if let Some(raster) = raster {
                match engine.recognize(&raster) {
                    Ok(data) => OcrState::Recognized(data),
                    Err(err) => OcrState::Failed(err.to_string()),
                }
            } else {
                OcrState::Failed("subtitle object missing from frame".to_owned())
            };

            OcrFrame {
                pts: frame.pts,
                backend: engine.name(),
                subtitle_size,
                state,
            }
        })
        .collect()
}

fn rasterize_subtitle(frame: &DisplaySet) -> Option<SubtitleRaster> {
    let ods = &frame.ods;
    let _obj = frame.pcs.find_object_by_id(ods.id)?;
    let width = u32::from(ods.width);
    let height = u32::from(ods.height);
    let mut pixels = vec![0_u8; width as usize * height as usize * 4];

    for (index, color_id) in ods.data.iter().copied().enumerate() {
        let rgba = if color_id == 0 {
            TRANSPARENT
        } else {
            frame
                .pds
                .find_by_id(color_id)
                .map(|entry| entry.rgba())
                .unwrap_or(TRANSPARENT)
        };

        let offset = index * 4;
        pixels[offset] = (rgba[0] * 255.0).round() as u8;
        pixels[offset + 1] = (rgba[1] * 255.0).round() as u8;
        pixels[offset + 2] = (rgba[2] * 255.0).round() as u8;
        pixels[offset + 3] = (rgba[3] * 255.0).round() as u8;
    }

    Some(SubtitleRaster {
        width,
        height,
        pixels,
    })
}

fn raster_to_luma(raster: &SubtitleRaster) -> Vec<u8> {
    raster
        .pixels
        .chunks_exact(4)
        .map(|rgba| {
            let alpha = rgba[3] as f32 / 255.0;
            if alpha == 0.0 {
                0
            } else {
                let luma =
                    0.2126 * rgba[0] as f32 + 0.7152 * rgba[1] as f32 + 0.0722 * rgba[2] as f32;
                (luma * alpha).round() as u8
            }
        })
        .collect()
}

fn parse_tesseract_tsv(tsv: &str) -> Result<OcrData> {
    let mut words = Vec::new();
    let mut lines = Vec::<(String, Vec<String>)>::new();

    for line in tsv.lines().skip(1) {
        if line.trim().is_empty() {
            continue;
        }

        let columns: Vec<_> = line.split('\t').collect();

        if columns.len() < 12 || columns[0] != "5" {
            continue;
        }

        let text = columns[11].trim();
        if text.is_empty() {
            continue;
        }

        let confidence = columns[10].parse::<f32>().ok().filter(|conf| *conf >= 0.0);
        let line_key = format!(
            "{}:{}:{}:{}",
            columns[1], columns[2], columns[3], columns[4]
        );

        match lines.last_mut() {
            Some((key, line_words)) if *key == line_key => line_words.push(text.to_owned()),
            _ => lines.push((line_key, vec![text.to_owned()])),
        }

        words.push(OcrWord {
            text: text.to_owned(),
            confidence,
        });
    }

    let text = lines
        .into_iter()
        .map(|(_, words)| words.join(" "))
        .collect::<Vec<_>>()
        .join("\n");

    let confidences: Vec<_> = words.iter().filter_map(|word| word.confidence).collect();
    let mean_confidence = if confidences.is_empty() {
        None
    } else {
        Some(confidences.iter().sum::<f32>() / confidences.len() as f32)
    };

    Ok(OcrData {
        text,
        mean_confidence,
        words,
    })
}

fn temp_ocr_path(extension: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    env::temp_dir().join(format!(
        "sup-decode-ocr-{}-{}.{}",
        std::process::id(),
        nanos,
        extension
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tesseract_tsv_words_and_confidence() {
        let tsv = "\
level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext
5\t1\t1\t1\t1\t1\t10\t10\t30\t10\t95.5\tFrench,
5\t1\t1\t1\t1\t2\t45\t10\t20\t10\t89.0\tthe
5\t1\t1\t1\t1\t3\t70\t10\t60\t10\t92.0\tlanguage
5\t1\t1\t1\t2\t1\t10\t30\t20\t10\t85.0\tof
5\t1\t1\t1\t2\t2\t35\t30\t35\t10\t80.0\tlove
";

        let data = parse_tesseract_tsv(tsv).unwrap();

        assert_eq!("French, the language\nof love", data.text);
        assert_eq!(5, data.words.len());
        assert_eq!(
            Some(88.3),
            data.mean_confidence.map(|x| (x * 10.0).round() / 10.0)
        );
        assert_eq!("French,", data.words[0].text);
        assert_eq!(Some(95.5), data.words[0].confidence);
    }

    #[test]
    fn ignores_non_word_rows_and_negative_confidence() {
        let tsv = "\
level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext
1\t1\t0\t0\t0\t0\t0\t0\t100\t50\t-1\t
5\t1\t1\t1\t1\t1\t10\t10\t30\t10\t-1\tHello
5\t1\t1\t1\t1\t2\t45\t10\t30\t10\t90\tworld
";

        let data = parse_tesseract_tsv(tsv).unwrap();

        assert_eq!("Hello world", data.text);
        assert_eq!(2, data.words.len());
        assert_eq!(Some(90.0), data.mean_confidence);
        assert_eq!(None, data.words[0].confidence);
    }
}
