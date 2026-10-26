use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
#[repr(i32)]
pub enum Transform {
    #[default]
    Normal = 0,
    Rotate90 = 1,
    Rotate180 = 2,
    Rotate270 = 3,
    Flipped = 4,
    Flipped90 = 5,
    Flipped180 = 6,
    Flipped270 = 7,
}

impl Transform {
    pub fn is_rotated_90(&self) -> bool {
        matches!(
            self,
            Transform::Rotate90
                | Transform::Rotate270
                | Transform::Flipped90
                | Transform::Flipped270
        )
    }

    pub fn from_i32(val: i32) -> Self {
        match val {
            1 => Transform::Rotate90,
            2 => Transform::Rotate180,
            3 => Transform::Rotate270,
            4 => Transform::Flipped,
            5 => Transform::Flipped90,
            6 => Transform::Flipped180,
            7 => Transform::Flipped270,
            _ => Transform::Normal,
        }
    }
}

/// Logical rectangle on the Wayland compositor desktop canvas
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogicalRect {
    pub origin: Point,
    pub width: f64,
    pub height: f64,
}

impl LogicalRect {
    pub fn new(x: i32, y: i32, width: f64, height: f64) -> Self {
        Self {
            origin: Point::new(x, y),
            width,
            height,
        }
    }

    pub fn right(&self) -> f64 {
        self.origin.x as f64 + self.width
    }

    pub fn bottom(&self) -> f64 {
        self.origin.y as f64 + self.height
    }

    pub fn overlaps(&self, other: &LogicalRect) -> bool {
        (self.origin.x as f64) < other.right()
            && self.right() > (other.origin.x as f64)
            && (self.origin.y as f64) < other.bottom()
            && self.bottom() > (other.origin.y as f64)
    }

    /// Snaps this rectangle to an adjacent target rectangle if edges fall within `threshold`
    pub fn snap_to(&mut self, target: &LogicalRect, threshold: f64) -> bool {
        let mut snapped = false;

        // Horizontal snapping: Left edge to target Right edge
        if (self.origin.x as f64 - target.right()).abs() <= threshold {
            self.origin.x = target.right().round() as i32;
            snapped = true;
        }
        // Horizontal snapping: Right edge to target Left edge
        else if (self.right() - target.origin.x as f64).abs() <= threshold {
            self.origin.x = (target.origin.x as f64 - self.width).round() as i32;
            snapped = true;
        }

        // Vertical snapping: Top edge to target Bottom edge
        if (self.origin.y as f64 - target.bottom()).abs() <= threshold {
            self.origin.y = target.bottom().round() as i32;
            snapped = true;
        }
        // Vertical snapping: Bottom edge to target Top edge
        else if (self.bottom() - target.origin.y as f64).abs() <= threshold {
            self.origin.y = (target.origin.y as f64 - self.height).round() as i32;
            snapped = true;
        }

        // Edge alignment snapping (top to top, bottom to bottom)
        if (self.origin.y as f64 - target.origin.y as f64).abs() <= threshold {
            self.origin.y = target.origin.y;
            snapped = true;
        } else if (self.bottom() - target.bottom()).abs() <= threshold {
            self.origin.y = (target.bottom() - self.height).round() as i32;
            snapped = true;
        }

        // Edge alignment snapping (left to left, right to right)
        if (self.origin.x as f64 - target.origin.x as f64).abs() <= threshold {
            self.origin.x = target.origin.x;
            snapped = true;
        } else if (self.right() - target.right()).abs() <= threshold {
            self.origin.x = (target.right() - self.width).round() as i32;
            snapped = true;
        }

        snapped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_overlap() {
        let r1 = LogicalRect::new(0, 0, 1920.0, 1080.0);
        let r2 = LogicalRect::new(1920, 0, 1920.0, 1080.0); // side-by-side, touching but not overlapping
        assert!(!r1.overlaps(&r2));

        let r3 = LogicalRect::new(1900, 0, 1920.0, 1080.0); // 20px overlap
        assert!(r1.overlaps(&r3));
    }

    #[test]
    fn test_snapping_to_adjacent_edge() {
        let target = LogicalRect::new(0, 0, 1920.0, 1080.0);
        let mut candidate = LogicalRect::new(1925, 4, 1920.0, 1080.0);

        let snapped = candidate.snap_to(&target, 16.0);
        assert!(snapped);
        assert_eq!(candidate.origin.x, 1920); // Snapped right edge of target
        assert_eq!(candidate.origin.y, 0);    // Snapped top edge of target
    }
}
