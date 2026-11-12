use core::f32::consts::{PI, TAU};
use core::ops::{Add, AddAssign, Neg, Sub, SubAssign};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use super::point::Point2D;

/// 2D robot pose (`x`, `y` in meters, `theta` in radians wrapped to `[-π, π]`).
///
/// Arithmetic operators (`+`, `-`, `+=`, `-=`, unary `-`) operate component-wise on `(x, y)`
/// and normalize `theta` into `[-π, π]`.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct Pose2D {
    pub x: f32,
    pub y: f32,
    pub theta: f32,
}

impl Pose2D {
    /// Creates a new pose, wrapping `theta` into `[-π, π]`.
    pub fn new(x: f32, y: f32, theta: f32) -> Self {
        Self {
            x,
            y,
            theta: wrap_angle(theta),
        }
    }

    /// Extracts the Cartesian `(x, y)` position as a [`Point2D`].
    pub const fn position(&self) -> Point2D {
        Point2D {
            x: self.x,
            y: self.y,
        }
    }
}

impl From<Pose2D> for Point2D {
    fn from(pose: Pose2D) -> Self {
        pose.position()
    }
}

impl From<&Pose2D> for Point2D {
    fn from(pose: &Pose2D) -> Self {
        pose.position()
    }
}

/// Normalizes an angle in radians to the interval `[-π, π)`.
pub fn wrap_angle(angle: f32) -> f32 {
    let mut a = (angle + PI) % TAU;
    if a < 0.0 {
        a += TAU;
    }
    a - PI
}

macro_rules! impl_pose_binop {
    ($trait:ident, $method:ident, $assign_trait:ident, $assign_method:ident, $op:tt) => {
        impl $trait<&Pose2D> for &Pose2D {
            type Output = Pose2D;

            fn $method(self, rhs: &Pose2D) -> Pose2D {
                Pose2D {
                    x: self.x $op rhs.x,
                    y: self.y $op rhs.y,
                    theta: wrap_angle(self.theta $op rhs.theta),
                }
            }
        }

        impl $trait<Pose2D> for Pose2D {
            type Output = Pose2D;

            fn $method(self, rhs: Pose2D) -> Pose2D {
                (&self).$method(&rhs)
            }
        }

        impl $trait<&Pose2D> for Pose2D {
            type Output = Pose2D;

            fn $method(self, rhs: &Pose2D) -> Pose2D {
                (&self).$method(rhs)
            }
        }

        impl $trait<Pose2D> for &Pose2D {
            type Output = Pose2D;

            fn $method(self, rhs: Pose2D) -> Pose2D {
                self.$method(&rhs)
            }
        }

        impl $assign_trait<Pose2D> for Pose2D {
            fn $assign_method(&mut self, rhs: Pose2D) {
                *self = (&*self).$method(&rhs);
            }
        }

        impl $assign_trait<&Pose2D> for Pose2D {
            fn $assign_method(&mut self, rhs: &Pose2D) {
                *self = (&*self).$method(rhs);
            }
        }
    };
}

impl_pose_binop!(Add, add, AddAssign, add_assign, +);
impl_pose_binop!(Sub, sub, SubAssign, sub_assign, -);

impl Neg for &Pose2D {
    type Output = Pose2D;

    fn neg(self) -> Pose2D {
        Pose2D {
            x: -self.x,
            y: -self.y,
            theta: wrap_angle(-self.theta),
        }
    }
}

impl Neg for Pose2D {
    type Output = Pose2D;

    fn neg(self) -> Pose2D {
        -&self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn pose2d_add_sub_and_refs() {
        let a = Pose2D {
            x: 1.0,
            y: 2.0,
            theta: 2.5,
        };
        let b = Pose2D {
            x: 0.5,
            y: -1.0,
            theta: 1.5,
        };

        let expected_add_theta = 4.0 - TAU;
        for sum in [a + b, a + &b, &a + b, &a + &b] {
            assert!(approx_eq(sum.x, 1.5));
            assert!(approx_eq(sum.y, 1.0));
            assert!(approx_eq(sum.theta, expected_add_theta));
        }

        let c = Pose2D {
            x: 1.0,
            y: 2.0,
            theta: -2.5,
        };
        let expected_sub_theta = -4.0 + TAU;
        for diff in [c - b, c - &b, &c - b, &c - &b] {
            assert!(approx_eq(diff.x, 0.5));
            assert!(approx_eq(diff.y, 3.0));
            assert!(approx_eq(diff.theta, expected_sub_theta));
        }

        let mut p = a;
        p += &b;
        assert!(approx_eq(p.x, 1.5));
        assert!(approx_eq(p.theta, expected_add_theta));
        p -= b;
        assert!(approx_eq(p.x, a.x));
        assert!(approx_eq(p.y, a.y));
        assert!(approx_eq(p.theta, a.theta));

        let neg = -&a;
        assert!(approx_eq(neg.x, -1.0));
        assert!(approx_eq(neg.y, -2.0));
        assert!(approx_eq(neg.theta, -2.5));
    }
}
