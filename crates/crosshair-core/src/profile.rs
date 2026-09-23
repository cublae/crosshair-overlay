use serde::{Deserialize, Serialize};

use crate::Rgba;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Shape {
    /// Four arms along the axes, split by `gap`.
    Cross,
    /// Left, right and bottom arms — the classic T.
    T,
    /// Four diagonal arms.
    X,
    /// Single ring of radius `length`.
    Circle,
    /// Axis-aligned square with half-size `length`.
    Square,
    /// Nothing but the centre dot.
    Dot,
}

impl Shape {
    pub const ALL: [Shape; 6] = [
        Shape::Cross,
        Shape::T,
        Shape::X,
        Shape::Circle,
        Shape::Square,
        Shape::Dot,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Shape::Cross => "cross",
            Shape::T => "t",
            Shape::X => "x",
            Shape::Circle => "circle",
            Shape::Square => "square",
            Shape::Dot => "dot",
        }
    }
}

/// Contrast outline stroked underneath the crosshair itself.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Outline {
    pub enabled: bool,
    /// Width added on *each* side of the main stroke.
    pub width: f64,
    pub color: Rgba,
}

impl Default for Outline {
    fn default() -> Self {
        Self {
            enabled: true,
            width: 1.0,
            color: Rgba::new(0.0, 0.0, 0.0, 0.85),
        }
    }
}

/// Centre dot. `color: None` inherits the crosshair colour.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Dot {
    pub enabled: bool,
    /// Radius in logical pixels.
    pub size: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Rgba>,
}

impl Default for Dot {
    fn default() -> Self {
        Self {
            enabled: false,
            size: 1.5,
            color: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub shape: Shape,
    pub color: Rgba,
    /// Multiplied into every alpha, so one slider dims the whole crosshair.
    pub opacity: f64,
    pub thickness: f64,
    /// Arm length for Cross/T/X, radius for Circle, half-size for Square.
    pub length: f64,
    /// Empty space between the centre and where the arms start.
    pub gap: f64,
    /// Degrees, clockwise, around the crosshair centre.
    pub rotation: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    pub outline: Outline,
    pub dot: Dot,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            shape: Shape::Cross,
            color: Rgba::new(0.0, 1.0, 0.0, 1.0),
            opacity: 1.0,
            thickness: 2.0,
            length: 10.0,
            gap: 4.0,
            rotation: 0.0,
            offset_x: 0.0,
            offset_y: 0.0,
            outline: Outline::default(),
            dot: Dot::default(),
        }
    }
}

impl Profile {
    /// Half-extent of the drawn shape, used to size the preview widget.
    pub fn radius(&self) -> f64 {
        let reach = match self.shape {
            Shape::Cross | Shape::T | Shape::X => self.gap + self.length,
            Shape::Circle | Shape::Square => self.length,
            Shape::Dot => 0.0,
        };
        let dot = if self.dot.enabled { self.dot.size } else { 0.0 };
        let pad = self.thickness / 2.0
            + if self.outline.enabled {
                self.outline.width
            } else {
                0.0
            };
        reach.max(dot) + pad
    }
}
