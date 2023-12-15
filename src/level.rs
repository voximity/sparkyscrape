use byteorder::{ReadBytesExt, LE};
use colored::Colorize;
use image::imageops::FilterType;
use rustdct::DctPlanner;
use std::{
    fmt::Display,
    io::{self, Cursor, Read},
};
use tokio::io::AsyncWriteExt;

pub const NUM_COEFFICIENTS: usize = 10;
pub type Coefficients = [f32; NUM_COEFFICIENTS];

pub struct Level {
    pub name: String,
    pub difficulty: LevelDifficulty,
    pub coefficients: Coefficients,
}

#[derive(Debug, Copy, Clone)]
pub enum LevelDifficulty {
    Easy,
    Medium,
    Hard,
    Legendary,
}

impl Display for LevelDifficulty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Easy => "Easy".bold().green(),
                Self::Medium => "Medium".bold().yellow(),
                Self::Hard => "Hard".bold().red(),
                Self::Legendary => "Legendary".bold().bright_yellow(),
            }
        )
    }
}

impl Level {
    pub fn image_coefficients(data: &[u8]) -> anyhow::Result<Coefficients> {
        let mut signal = image::io::Reader::new(Cursor::new(data))
            .with_guessed_format()?
            .decode()?
            .resize_exact(128, 128, FilterType::Triangle)
            .into_luma8()
            .iter()
            .map(|b| *b as f32)
            .collect::<Vec<f32>>();

        // TODO: reuse DCT planners
        let mut planner = DctPlanner::new();
        let dct = planner.plan_dct2(signal.len());
        dct.process_dct2(&mut signal);

        Ok(signal[0..NUM_COEFFICIENTS].try_into()?)
    }

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

        let mut coefficients = [0f32; NUM_COEFFICIENTS];
        reader.read_f32_into::<LE>(&mut coefficients)?;

        Ok(Self {
            name,
            difficulty,
            coefficients,
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

        for coeff in &self.coefficients {
            writer.write_f32_le(*coeff).await?;
        }

        Ok(())
    }

    pub fn _cosine_similarity_with(&self, other: &[f32; NUM_COEFFICIENTS]) -> f32 {
        let dot = self
            .coefficients
            .iter()
            .zip(other)
            .fold(0f32, |acc, (a, b)| acc + a * b);
        let mag = self.coefficients.iter().map(|a| a * a).sum::<f32>().sqrt()
            * other.iter().map(|b| b * b).sum::<f32>().sqrt();

        dot / mag
    }

    // pub fn euclidean_distance_to(&self, &other: &[f32; NUM_COEFFICIENTS]) -> f32 {
    //     self.coefficients
    //         .iter()
    //         .zip(other)
    //         .fold(0f32, |acc, (a, b)| acc + (a - b).powi(2))
    //         .sqrt()
    // }

    pub fn euclidean_distance_to(&self, &other: &[f32; NUM_COEFFICIENTS]) -> f32 {
        let mut acc = 0f32;
        for (i, (a, b)) in self.coefficients.iter().zip(other).enumerate() {
            acc += ((NUM_COEFFICIENTS - i) as f32) / (NUM_COEFFICIENTS as f32) * (a - b).powi(2)
        }
        acc.sqrt()
    }
}
