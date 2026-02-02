//! Vector types

use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// A 2D vector
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Vec2 {
    /// X component
    pub x: f32,
    /// Y component
    pub y: f32,
}

impl Vec2 {
    /// Zero vector
    pub const ZERO: Self = Self::new(0.0, 0.0);
    /// One vector
    pub const ONE: Self = Self::new(1.0, 1.0);
    /// X unit vector
    pub const X: Self = Self::new(1.0, 0.0);
    /// Y unit vector
    pub const Y: Self = Self::new(0.0, 1.0);

    /// Creates a new vector
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Creates a vector with all components set to the same value
    #[inline]
    pub const fn splat(v: f32) -> Self {
        Self::new(v, v)
    }

    /// Dot product
    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    /// Length squared
    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    /// Length
    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Normalize
    #[inline]
    pub fn normalize(self) -> Self {
        self / self.length()
    }

    /// Linear interpolation
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }

    /// Extends to Vec3
    #[inline]
    pub const fn extend(self, z: f32) -> Vec3 {
        Vec3::new(self.x, self.y, z)
    }
}

impl Add for Vec2 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Sub for Vec2 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f32) -> Self {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl Neg for Vec2 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y)
    }
}

/// A 3D vector
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Vec3 {
    /// X component
    pub x: f32,
    /// Y component
    pub y: f32,
    /// Z component
    pub z: f32,
}

impl Vec3 {
    /// Zero vector
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    /// One vector
    pub const ONE: Self = Self::new(1.0, 1.0, 1.0);
    /// X unit vector
    pub const X: Self = Self::new(1.0, 0.0, 0.0);
    /// Y unit vector
    pub const Y: Self = Self::new(0.0, 1.0, 0.0);
    /// Z unit vector
    pub const Z: Self = Self::new(0.0, 0.0, 1.0);

    /// Creates a new vector
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Creates a vector with all components set to the same value
    #[inline]
    pub const fn splat(v: f32) -> Self {
        Self::new(v, v, v)
    }

    /// Dot product
    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Cross product
    #[inline]
    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    /// Length squared
    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    /// Length
    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Normalize
    #[inline]
    pub fn normalize(self) -> Self {
        self / self.length()
    }

    /// Linear interpolation
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }

    /// Extends to Vec4
    #[inline]
    pub const fn extend(self, w: f32) -> Vec4 {
        Vec4::new(self.x, self.y, self.z, w)
    }

    /// Truncates to Vec2
    #[inline]
    pub const fn truncate(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
}

impl Add for Vec3 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl AddAssign for Vec3 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Vec3 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl SubAssign for Vec3 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl MulAssign<f32> for Vec3 {
    #[inline]
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}

impl Div<f32> for Vec3 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f32) -> Self {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

impl DivAssign<f32> for Vec3 {
    #[inline]
    fn div_assign(&mut self, rhs: f32) {
        *self = *self / rhs;
    }
}

impl Neg for Vec3 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

/// A 4D vector
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Vec4 {
    /// X component
    pub x: f32,
    /// Y component
    pub y: f32,
    /// Z component
    pub z: f32,
    /// W component
    pub w: f32,
}

impl Vec4 {
    /// Zero vector
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0, 0.0);
    /// One vector
    pub const ONE: Self = Self::new(1.0, 1.0, 1.0, 1.0);
    /// X unit vector
    pub const X: Self = Self::new(1.0, 0.0, 0.0, 0.0);
    /// Y unit vector
    pub const Y: Self = Self::new(0.0, 1.0, 0.0, 0.0);
    /// Z unit vector
    pub const Z: Self = Self::new(0.0, 0.0, 1.0, 0.0);
    /// W unit vector
    pub const W: Self = Self::new(0.0, 0.0, 0.0, 1.0);

    /// Creates a new vector
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// Creates a vector with all components set to the same value
    #[inline]
    pub const fn splat(v: f32) -> Self {
        Self::new(v, v, v, v)
    }

    /// Dot product
    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }

    /// Length squared
    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    /// Length
    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Normalize
    #[inline]
    pub fn normalize(self) -> Self {
        self / self.length()
    }

    /// Linear interpolation
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }

    /// Truncates to Vec3
    #[inline]
    pub const fn truncate(self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }

    /// Returns xyz as Vec3
    #[inline]
    pub const fn xyz(self) -> Vec3 {
        self.truncate()
    }
}

impl Add for Vec4 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(
            self.x + rhs.x,
            self.y + rhs.y,
            self.z + rhs.z,
            self.w + rhs.w,
        )
    }
}

impl Sub for Vec4 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(
            self.x - rhs.x,
            self.y - rhs.y,
            self.z - rhs.z,
            self.w - rhs.w,
        )
    }
}

impl Mul<f32> for Vec4 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs, self.w * rhs)
    }
}

impl Div<f32> for Vec4 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f32) -> Self {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs, self.w / rhs)
    }
}

impl Neg for Vec4 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z, -self.w)
    }
}

impl From<[f32; 4]> for Vec4 {
    #[inline]
    fn from(arr: [f32; 4]) -> Self {
        Self::new(arr[0], arr[1], arr[2], arr[3])
    }
}

impl From<Vec4> for [f32; 4] {
    #[inline]
    fn from(v: Vec4) -> Self {
        [v.x, v.y, v.z, v.w]
    }
}
