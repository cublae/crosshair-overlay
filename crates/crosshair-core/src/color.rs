use std::fmt;

use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

/// Colour with straight alpha, components in `0.0..=1.0`.
///
/// Serialised as `#rrggbb` when fully opaque, `#rrggbbaa` otherwise.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Rgba {
    pub const fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);

    /// Multiplies alpha by `factor`, used for the global opacity slider.
    pub fn faded(self, factor: f64) -> Self {
        Self {
            a: (self.a * factor).clamp(0.0, 1.0),
            ..self
        }
    }

    pub fn to_hex(self) -> String {
        let c = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        if self.a >= 1.0 {
            format!("#{:02x}{:02x}{:02x}", c(self.r), c(self.g), c(self.b))
        } else {
            format!(
                "#{:02x}{:02x}{:02x}{:02x}",
                c(self.r),
                c(self.g),
                c(self.b),
                c(self.a)
            )
        }
    }

    pub fn parse(s: &str) -> Result<Self, ParseColorError> {
        let hex = s.strip_prefix('#').unwrap_or(s);
        let byte = |i: usize| -> Result<f64, ParseColorError> {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map(|v| f64::from(v) / 255.0)
                .map_err(|_| ParseColorError(s.to_owned()))
        };
        match hex.len() {
            6 => Ok(Self::new(byte(0)?, byte(2)?, byte(4)?, 1.0)),
            8 => Ok(Self::new(byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
            _ => Err(ParseColorError(s.to_owned())),
        }
    }
}

#[derive(Debug)]
pub struct ParseColorError(String);

impl fmt::Display for ParseColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid colour {:?}, expected #rrggbb or #rrggbbaa",
            self.0
        )
    }
}

impl std::error::Error for ParseColorError {}

impl Serialize for Rgba {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Rgba {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        Self::parse(&raw).map_err(D::Error::custom)
    }
}
