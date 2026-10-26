use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Resolution {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn aspect_ratio(&self) -> f64 {
        if self.height == 0 {
            0.0
        } else {
            self.width as f64 / self.height as f64
        }
    }
}

impl fmt::Display for Resolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Mode {
    pub resolution: Resolution,
    pub refresh_rate: f64,
}

impl Mode {
    pub fn new(width: u32, height: u32, refresh_rate: f64) -> Self {
        Self {
            resolution: Resolution::new(width, height),
            refresh_rate,
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if (self.refresh_rate.fract()).abs() < 0.001 {
            write!(f, "{}@{:.0}Hz", self.resolution, self.refresh_rate)
        } else {
            write!(f, "{}@{:.2}Hz", self.resolution, self.refresh_rate)
        }
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum ModeParseError {
    #[error("Invalid format, expected WxH or WxH@Hz")]
    InvalidFormat,
    #[error("Invalid integer in resolution: {0}")]
    InvalidInt(#[from] std::num::ParseIntError),
    #[error("Invalid float in refresh rate: {0}")]
    InvalidFloat(#[from] std::num::ParseFloatError),
}

impl FromStr for Mode {
    type Err = ModeParseError;

    /// Parses modes formatted as "1920x1080@144.00Hz", "1920x1080@60Hz", "1920x1080@144", or "2560x1440"
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let clean = s.trim_end_matches("Hz").trim();
        let (res_part, rate_part) = match clean.split_once('@') {
            Some((res, rate)) => (res, Some(rate)),
            None => (clean, None),
        };

        let (w_str, h_str) = res_part
            .split_once('x')
            .ok_or(ModeParseError::InvalidFormat)?;

        let width = w_str.trim().parse::<u32>()?;
        let height = h_str.trim().parse::<u32>()?;
        let refresh_rate = match rate_part {
            Some(r) => r.trim().parse::<f64>()?,
            None => 60.0,
        };

        Ok(Mode {
            resolution: Resolution::new(width, height),
            refresh_rate,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_from_str() {
        let m: Mode = "1920x1080@144.00Hz".parse().unwrap();
        assert_eq!(m.resolution.width, 1920);
        assert_eq!(m.resolution.height, 1080);
        assert!((m.refresh_rate - 144.00).abs() < 0.001);

        let m2: Mode = "2560x1440@59.95".parse().unwrap();
        assert_eq!(m2.resolution.width, 2560);
        assert_eq!(m2.resolution.height, 1440);
        assert!((m2.refresh_rate - 59.95).abs() < 0.001);

        let m3: Mode = "3840x2160".parse().unwrap();
        assert_eq!(m3.resolution.width, 3840);
        assert_eq!(m3.resolution.height, 2160);
        assert!((m3.refresh_rate - 60.0).abs() < 0.001);
    }

    #[test]
    fn test_mode_display() {
        let m = Mode::new(1920, 1080, 144.0);
        assert_eq!(m.to_string(), "1920x1080@144Hz");

        let m2 = Mode::new(2560, 1440, 59.95);
        assert_eq!(m2.to_string(), "2560x1440@59.95Hz");
    }
}
