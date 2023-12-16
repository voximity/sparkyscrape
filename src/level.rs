use byteorder::{ReadBytesExt, LE};
use colored::{ColoredString, Colorize};
use image::imageops::FilterType;
use rustdct::TransformType2And3;
use serde::Serialize;
use std::{
    fmt::Display,
    io::{self, Cursor, Read},
    sync::Arc,
};
use tokio::io::AsyncWriteExt;

pub const IMAGE_DIM: usize = 128;
pub const NUM_COEFFICIENTS: usize = 10;

#[derive(Debug, Default, Copy, Clone)]
pub struct Coefficients {
    pub r: [f32; NUM_COEFFICIENTS],
    pub g: [f32; NUM_COEFFICIENTS],
    pub b: [f32; NUM_COEFFICIENTS],
}

fn euclidean_distance(a: &[f32; NUM_COEFFICIENTS], b: &[f32; NUM_COEFFICIENTS]) -> f32 {
    let mut acc = 0f32;
    for (i, (a, b)) in a.iter().zip(b).enumerate() {
        acc += ((NUM_COEFFICIENTS - i) as f32) / (NUM_COEFFICIENTS as f32) * (a - b).powi(2)
    }
    acc.sqrt()
}

impl Coefficients {
    pub fn new(data: &[u8], dct: Arc<dyn TransformType2And3<f32>>) -> anyhow::Result<Self> {
        let img = image::io::Reader::new(Cursor::new(data))
            .with_guessed_format()?
            .decode()?
            .resize_exact(IMAGE_DIM as u32, IMAGE_DIM as u32, FilterType::Triangle)
            .into_rgb32f()
            .iter()
            .copied()
            .collect::<Vec<_>>();

        let mut r = vec![0f32; IMAGE_DIM * IMAGE_DIM];
        let mut g = vec![0f32; IMAGE_DIM * IMAGE_DIM];
        let mut b = vec![0f32; IMAGE_DIM * IMAGE_DIM];
        for (i, chunk) in img.chunks_exact(3).enumerate() {
            r[i] = chunk[0];
            g[i] = chunk[1];
            b[i] = chunk[2];
        }

        dct.process_dct2(&mut r);
        dct.process_dct2(&mut g);
        dct.process_dct2(&mut b);

        Ok(Self {
            r: r[0..NUM_COEFFICIENTS].try_into()?,
            g: g[0..NUM_COEFFICIENTS].try_into()?,
            b: b[0..NUM_COEFFICIENTS].try_into()?,
        })
    }
}

pub struct Level {
    pub name: String,
    pub difficulty: LevelDifficulty,
    pub coefficients: Coefficients,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum LevelDifficulty {
    Easy,
    Medium,
    Hard,
    Legendary,
}

impl Serialize for LevelDifficulty {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_i32(match self {
            Self::Easy => 0,
            Self::Medium => 1,
            Self::Hard => 2,
            Self::Legendary => 3,
        })
    }
}

impl Display for LevelDifficulty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.colorize(match self {
                Self::Easy => "Easy",
                Self::Medium => "Medium",
                Self::Hard => "Hard",
                Self::Legendary => "Legendary",
            })
            .bold()
        )
    }
}

impl LevelDifficulty {
    pub fn colorize(&self, s: impl Colorize) -> ColoredString {
        match self {
            Self::Easy => s.green(),
            Self::Medium => s.yellow(),
            Self::Hard => s.red(),
            Self::Legendary => s.purple(),
        }
    }

    pub fn directory(&self) -> &'static str {
        match self {
            Self::Easy => "easy",
            Self::Medium => "medium",
            Self::Hard => "hard",
            Self::Legendary => "legendary",
        }
    }

    pub fn filename(&self) -> &'static str {
        match self {
            Self::Easy => "easy.bin",
            Self::Medium => "medium.bin",
            Self::Hard => "hard.bin",
            Self::Legendary => "legendary.bin",
        }
    }
}

impl Level {
    pub fn read<R: Read>(reader: &mut R) -> io::Result<Self> {
        let name_len = reader.read_u8()?;
        let mut name = String::new();

        for _ in 0..name_len {
            name.push(reader.read_u8()? as char);
        }

        let difficulty = match reader.read_u8()? {
            1 => LevelDifficulty::Medium,
            2 => LevelDifficulty::Hard,
            3 => LevelDifficulty::Legendary,
            _ => LevelDifficulty::Easy,
        };

        let mut r = [0f32; NUM_COEFFICIENTS];
        let mut g = [0f32; NUM_COEFFICIENTS];
        let mut b = [0f32; NUM_COEFFICIENTS];
        reader.read_f32_into::<LE>(&mut r)?;
        reader.read_f32_into::<LE>(&mut g)?;
        reader.read_f32_into::<LE>(&mut b)?;

        Ok(Self {
            name,
            difficulty,
            coefficients: Coefficients { r, g, b },
        })
    }

    pub async fn write(&self, writer: &mut tokio::fs::File) -> io::Result<()> {
        writer.write_u8(self.name.len() as u8).await?;
        for c in self.name.chars() {
            writer.write_u8(c as u8).await?;
        }

        writer
            .write_u8(match self.difficulty {
                LevelDifficulty::Easy => 0,
                LevelDifficulty::Medium => 1,
                LevelDifficulty::Hard => 2,
                LevelDifficulty::Legendary => 3,
            })
            .await?;

        for coeff in &self.coefficients.r {
            writer.write_f32_le(*coeff).await?;
        }

        for coeff in &self.coefficients.g {
            writer.write_f32_le(*coeff).await?;
        }

        for coeff in &self.coefficients.b {
            writer.write_f32_le(*coeff).await?;
        }

        Ok(())
    }

    pub fn euclidean_distance_to(&self, &other: &Coefficients) -> f32 {
        let r = euclidean_distance(&self.coefficients.r, &other.r);
        let g = euclidean_distance(&self.coefficients.g, &other.g);
        let b = euclidean_distance(&self.coefficients.b, &other.b);

        // TODO: is average the best way to do this?
        (r + g + b) / 3f32
    }
}
