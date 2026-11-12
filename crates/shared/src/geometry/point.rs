use core::ops::{Add, AddAssign, Neg, Sub, SubAssign};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use super::pose::wrap_angle;

/// 2D Cartesian point or displacement vector in meters (`+X` forward, `+Y` left).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct Point2D {
    pub x: f32,
    pub y: f32,
}

/// 2D polar coordinate in standard counter-clockwise (CCW) convention, with `theta` in `[-π, π]`.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct PointPolar {
    pub r: f32,
    pub theta: f32,
}

impl Point2D {
    /// Creates a new Cartesian 2D point.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[cfg(feature = "std")]
    /// Returns the Euclidean norm `hypot(x, y)` in meters.
    pub fn norm(self) -> f32 {
        self.x.hypot(self.y)
    }

    #[cfg(feature = "std")]
    /// Returns the Euclidean distance between `self` and `other` in meters.
    pub fn distance_to(self, other: Self) -> f32 {
        (self - other).norm()
    }
}

impl PointPolar {
    /// Creates a polar point, wrapping `theta` into `[-π, π]`.
    pub fn new(r: f32, theta: f32) -> Self {
        Self {
            r,
            theta: wrap_angle(theta),
        }
    }
}

impl From<[f32; 2]> for Point2D {
    fn from([x, y]: [f32; 2]) -> Self {
        Self { x, y }
    }
}

impl From<Point2D> for [f32; 2] {
    fn from(p: Point2D) -> Self {
        [p.x, p.y]
    }
}

#[cfg(feature = "std")]
impl From<PointPolar> for Point2D {
    fn from(p: PointPolar) -> Self {
        let (sin, cos) = p.theta.sin_cos();
        Self {
            x: p.r * cos,
            y: p.r * sin,
        }
    }
}

#[cfg(feature = "std")]
impl From<&PointPolar> for Point2D {
    fn from(p: &PointPolar) -> Self {
        Self::from(*p)
    }
}

#[cfg(feature = "std")]
impl From<Point2D> for PointPolar {
    fn from(p: Point2D) -> Self {
        Self {
            r: p.norm(),
            theta: p.y.atan2(p.x),
        }
    }
}

#[cfg(feature = "std")]
impl From<&Point2D> for PointPolar {
    fn from(p: &Point2D) -> Self {
        Self::from(*p)
    }
}

macro_rules! impl_point2d_binop {
    ($trait:ident, $method:ident, $assign_trait:ident, $assign_method:ident, $op:tt) => {
        impl $trait<&Point2D> for &Point2D {
            type Output = Point2D;

            fn $method(self, rhs: &Point2D) -> Point2D {
                Point2D {
                    x: self.x $op rhs.x,
                    y: self.y $op rhs.y,
                }
            }
        }

        impl $trait<Point2D> for Point2D {
            type Output = Point2D;

            fn $method(self, rhs: Point2D) -> Point2D {
                (&self).$method(&rhs)
            }
        }

        impl $trait<&Point2D> for Point2D {
            type Output = Point2D;

            fn $method(self, rhs: &Point2D) -> Point2D {
                (&self).$method(rhs)
            }
        }

        impl $trait<Point2D> for &Point2D {
            type Output = Point2D;

            fn $method(self, rhs: Point2D) -> Point2D {
                self.$method(&rhs)
            }
        }

        impl $assign_trait<Point2D> for Point2D {
            fn $assign_method(&mut self, rhs: Point2D) {
                *self = (&*self).$method(&rhs);
            }
        }

        impl $assign_trait<&Point2D> for Point2D {
            fn $assign_method(&mut self, rhs: &Point2D) {
                *self = (&*self).$method(rhs);
            }
        }
    };
}

impl_point2d_binop!(Add, add, AddAssign, add_assign, +);
impl_point2d_binop!(Sub, sub, SubAssign, sub_assign, -);

impl Neg for &Point2D {
    type Output = Point2D;

    fn neg(self) -> Point2D {
        Point2D {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl Neg for Point2D {
    type Output = Point2D;

    fn neg(self) -> Point2D {
        -&self
    }
}
