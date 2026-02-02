//! Integer vector types
//!
//! This module provides integer versions of vector types commonly
//! used for texture coordinates, screen positions, and compute indices.

use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// A 2D integer vector
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct IVec2 {
    /// X component
    pub x: i32,
    /// Y component
    pub y: i32,
}

impl IVec2 {
    /// Zero vector
    pub const ZERO: Self = Self::new(0, 0);
    /// One vector
    pub const ONE: Self = Self::new(1, 1);
    /// X unit vector
    pub const X: Self = Self::new(1, 0);
    /// Y unit vector
    pub const Y: Self = Self::new(0, 1);
    /// Negative X unit vector
    pub const NEG_X: Self = Self::new(-1, 0);
    /// Negative Y unit vector
    pub const NEG_Y: Self = Self::new(0, -1);

    /// Creates a new vector
    #[inline]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Creates a vector with all components set to the same value
    #[inline]
    pub const fn splat(v: i32) -> Self {
        Self::new(v, v)
    }

    /// Converts to a float vector
    #[inline]
    pub fn as_vec2(self) -> crate::vec::Vec2 {
        crate::vec::Vec2::new(self.x as f32, self.y as f32)
    }

    /// Extends to IVec3
    #[inline]
    pub const fn extend(self, z: i32) -> IVec3 {
        IVec3::new(self.x, self.y, z)
    }

    /// Component-wise minimum
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self::new(self.x.min(other.x), self.y.min(other.y))
    }

    /// Component-wise maximum
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self::new(self.x.max(other.x), self.y.max(other.y))
    }

    /// Component-wise clamp
    #[inline]
    pub fn clamp(self, min: Self, max: Self) -> Self {
        self.max(min).min(max)
    }

    /// Component-wise absolute value
    #[inline]
    pub fn abs(self) -> Self {
        Self::new(self.x.abs(), self.y.abs())
    }

    /// Dot product
    #[inline]
    pub fn dot(self, other: Self) -> i32 {
        self.x * other.x + self.y * other.y
    }

    /// Length squared (returns i32 to avoid overflow issues)
    #[inline]
    pub fn length_squared(self) -> i32 {
        self.dot(self)
    }
}

impl Add for IVec2 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign for IVec2 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for IVec2 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl SubAssign for IVec2 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<i32> for IVec2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: i32) -> Self {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl MulAssign<i32> for IVec2 {
    #[inline]
    fn mul_assign(&mut self, rhs: i32) {
        *self = *self * rhs;
    }
}

impl Div<i32> for IVec2 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: i32) -> Self {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl DivAssign<i32> for IVec2 {
    #[inline]
    fn div_assign(&mut self, rhs: i32) {
        *self = *self / rhs;
    }
}

impl Neg for IVec2 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y)
    }
}

impl From<[i32; 2]> for IVec2 {
    #[inline]
    fn from(arr: [i32; 2]) -> Self {
        Self::new(arr[0], arr[1])
    }
}

impl From<IVec2> for [i32; 2] {
    #[inline]
    fn from(v: IVec2) -> Self {
        [v.x, v.y]
    }
}

impl From<(i32, i32)> for IVec2 {
    #[inline]
    fn from((x, y): (i32, i32)) -> Self {
        Self::new(x, y)
    }
}

/// A 3D integer vector
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct IVec3 {
    /// X component
    pub x: i32,
    /// Y component
    pub y: i32,
    /// Z component
    pub z: i32,
}

impl IVec3 {
    /// Zero vector
    pub const ZERO: Self = Self::new(0, 0, 0);
    /// One vector
    pub const ONE: Self = Self::new(1, 1, 1);
    /// X unit vector
    pub const X: Self = Self::new(1, 0, 0);
    /// Y unit vector
    pub const Y: Self = Self::new(0, 1, 0);
    /// Z unit vector
    pub const Z: Self = Self::new(0, 0, 1);
    /// Negative X unit vector
    pub const NEG_X: Self = Self::new(-1, 0, 0);
    /// Negative Y unit vector
    pub const NEG_Y: Self = Self::new(0, -1, 0);
    /// Negative Z unit vector
    pub const NEG_Z: Self = Self::new(0, 0, -1);

    /// Creates a new vector
    #[inline]
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Creates a vector with all components set to the same value
    #[inline]
    pub const fn splat(v: i32) -> Self {
        Self::new(v, v, v)
    }

    /// Converts to a float vector
    #[inline]
    pub fn as_vec3(self) -> crate::vec::Vec3 {
        crate::vec::Vec3::new(self.x as f32, self.y as f32, self.z as f32)
    }

    /// Extends to IVec4
    #[inline]
    pub const fn extend(self, w: i32) -> IVec4 {
        IVec4::new(self.x, self.y, self.z, w)
    }

    /// Truncates to IVec2
    #[inline]
    pub const fn truncate(self) -> IVec2 {
        IVec2::new(self.x, self.y)
    }

    /// Returns XY components
    #[inline]
    pub const fn xy(self) -> IVec2 {
        self.truncate()
    }

    /// Component-wise minimum
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self::new(
            self.x.min(other.x),
            self.y.min(other.y),
            self.z.min(other.z),
        )
    }

    /// Component-wise maximum
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self::new(
            self.x.max(other.x),
            self.y.max(other.y),
            self.z.max(other.z),
        )
    }

    /// Component-wise clamp
    #[inline]
    pub fn clamp(self, min: Self, max: Self) -> Self {
        self.max(min).min(max)
    }

    /// Component-wise absolute value
    #[inline]
    pub fn abs(self) -> Self {
        Self::new(self.x.abs(), self.y.abs(), self.z.abs())
    }

    /// Dot product
    #[inline]
    pub fn dot(self, other: Self) -> i32 {
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
    pub fn length_squared(self) -> i32 {
        self.dot(self)
    }
}

impl Add for IVec3 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl AddAssign for IVec3 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for IVec3 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl SubAssign for IVec3 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<i32> for IVec3 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: i32) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl MulAssign<i32> for IVec3 {
    #[inline]
    fn mul_assign(&mut self, rhs: i32) {
        *self = *self * rhs;
    }
}

impl Div<i32> for IVec3 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: i32) -> Self {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

impl DivAssign<i32> for IVec3 {
    #[inline]
    fn div_assign(&mut self, rhs: i32) {
        *self = *self / rhs;
    }
}

impl Neg for IVec3 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

impl From<[i32; 3]> for IVec3 {
    #[inline]
    fn from(arr: [i32; 3]) -> Self {
        Self::new(arr[0], arr[1], arr[2])
    }
}

impl From<IVec3> for [i32; 3] {
    #[inline]
    fn from(v: IVec3) -> Self {
        [v.x, v.y, v.z]
    }
}

impl From<(i32, i32, i32)> for IVec3 {
    #[inline]
    fn from((x, y, z): (i32, i32, i32)) -> Self {
        Self::new(x, y, z)
    }
}

/// A 4D integer vector
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct IVec4 {
    /// X component
    pub x: i32,
    /// Y component
    pub y: i32,
    /// Z component
    pub z: i32,
    /// W component
    pub w: i32,
}

impl IVec4 {
    /// Zero vector
    pub const ZERO: Self = Self::new(0, 0, 0, 0);
    /// One vector
    pub const ONE: Self = Self::new(1, 1, 1, 1);
    /// X unit vector
    pub const X: Self = Self::new(1, 0, 0, 0);
    /// Y unit vector
    pub const Y: Self = Self::new(0, 1, 0, 0);
    /// Z unit vector
    pub const Z: Self = Self::new(0, 0, 1, 0);
    /// W unit vector
    pub const W: Self = Self::new(0, 0, 0, 1);

    /// Creates a new vector
    #[inline]
    pub const fn new(x: i32, y: i32, z: i32, w: i32) -> Self {
        Self { x, y, z, w }
    }

    /// Creates a vector with all components set to the same value
    #[inline]
    pub const fn splat(v: i32) -> Self {
        Self::new(v, v, v, v)
    }

    /// Converts to a float vector
    #[inline]
    pub fn as_vec4(self) -> crate::vec::Vec4 {
        crate::vec::Vec4::new(self.x as f32, self.y as f32, self.z as f32, self.w as f32)
    }

    /// Truncates to IVec3
    #[inline]
    pub const fn truncate(self) -> IVec3 {
        IVec3::new(self.x, self.y, self.z)
    }

    /// Returns XYZ components
    #[inline]
    pub const fn xyz(self) -> IVec3 {
        self.truncate()
    }

    /// Returns XY components
    #[inline]
    pub const fn xy(self) -> IVec2 {
        IVec2::new(self.x, self.y)
    }

    /// Component-wise minimum
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self::new(
            self.x.min(other.x),
            self.y.min(other.y),
            self.z.min(other.z),
            self.w.min(other.w),
        )
    }

    /// Component-wise maximum
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self::new(
            self.x.max(other.x),
            self.y.max(other.y),
            self.z.max(other.z),
            self.w.max(other.w),
        )
    }

    /// Component-wise clamp
    #[inline]
    pub fn clamp(self, min: Self, max: Self) -> Self {
        self.max(min).min(max)
    }

    /// Component-wise absolute value
    #[inline]
    pub fn abs(self) -> Self {
        Self::new(self.x.abs(), self.y.abs(), self.z.abs(), self.w.abs())
    }

    /// Dot product
    #[inline]
    pub fn dot(self, other: Self) -> i32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }

    /// Length squared
    #[inline]
    pub fn length_squared(self) -> i32 {
        self.dot(self)
    }
}

impl Add for IVec4 {
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

impl AddAssign for IVec4 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for IVec4 {
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

impl SubAssign for IVec4 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<i32> for IVec4 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: i32) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs, self.w * rhs)
    }
}

impl MulAssign<i32> for IVec4 {
    #[inline]
    fn mul_assign(&mut self, rhs: i32) {
        *self = *self * rhs;
    }
}

impl Div<i32> for IVec4 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: i32) -> Self {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs, self.w / rhs)
    }
}

impl DivAssign<i32> for IVec4 {
    #[inline]
    fn div_assign(&mut self, rhs: i32) {
        *self = *self / rhs;
    }
}

impl Neg for IVec4 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z, -self.w)
    }
}

impl From<[i32; 4]> for IVec4 {
    #[inline]
    fn from(arr: [i32; 4]) -> Self {
        Self::new(arr[0], arr[1], arr[2], arr[3])
    }
}

impl From<IVec4> for [i32; 4] {
    #[inline]
    fn from(v: IVec4) -> Self {
        [v.x, v.y, v.z, v.w]
    }
}

impl From<(i32, i32, i32, i32)> for IVec4 {
    #[inline]
    fn from((x, y, z, w): (i32, i32, i32, i32)) -> Self {
        Self::new(x, y, z, w)
    }
}

/// A 2D unsigned integer vector
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct UVec2 {
    /// X component
    pub x: u32,
    /// Y component
    pub y: u32,
}

impl UVec2 {
    /// Zero vector
    pub const ZERO: Self = Self::new(0, 0);
    /// One vector
    pub const ONE: Self = Self::new(1, 1);
    /// X unit vector
    pub const X: Self = Self::new(1, 0);
    /// Y unit vector
    pub const Y: Self = Self::new(0, 1);

    /// Creates a new vector
    #[inline]
    pub const fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }

    /// Creates a vector with all components set to the same value
    #[inline]
    pub const fn splat(v: u32) -> Self {
        Self::new(v, v)
    }

    /// Converts to a float vector
    #[inline]
    pub fn as_vec2(self) -> crate::vec::Vec2 {
        crate::vec::Vec2::new(self.x as f32, self.y as f32)
    }

    /// Extends to UVec3
    #[inline]
    pub const fn extend(self, z: u32) -> UVec3 {
        UVec3::new(self.x, self.y, z)
    }

    /// Component-wise minimum
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self::new(self.x.min(other.x), self.y.min(other.y))
    }

    /// Component-wise maximum
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self::new(self.x.max(other.x), self.y.max(other.y))
    }

    /// Component-wise clamp
    #[inline]
    pub fn clamp(self, min: Self, max: Self) -> Self {
        self.max(min).min(max)
    }

    /// Dot product
    #[inline]
    pub fn dot(self, other: Self) -> u32 {
        self.x * other.x + self.y * other.y
    }

    /// Length squared
    #[inline]
    pub fn length_squared(self) -> u32 {
        self.dot(self)
    }

    /// Converts to signed integer vector
    #[inline]
    pub fn as_ivec2(self) -> IVec2 {
        IVec2::new(self.x as i32, self.y as i32)
    }
}

impl Add for UVec2 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign for UVec2 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for UVec2 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl SubAssign for UVec2 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<u32> for UVec2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: u32) -> Self {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl MulAssign<u32> for UVec2 {
    #[inline]
    fn mul_assign(&mut self, rhs: u32) {
        *self = *self * rhs;
    }
}

impl Div<u32> for UVec2 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: u32) -> Self {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl DivAssign<u32> for UVec2 {
    #[inline]
    fn div_assign(&mut self, rhs: u32) {
        *self = *self / rhs;
    }
}

impl From<[u32; 2]> for UVec2 {
    #[inline]
    fn from(arr: [u32; 2]) -> Self {
        Self::new(arr[0], arr[1])
    }
}

impl From<UVec2> for [u32; 2] {
    #[inline]
    fn from(v: UVec2) -> Self {
        [v.x, v.y]
    }
}

impl From<(u32, u32)> for UVec2 {
    #[inline]
    fn from((x, y): (u32, u32)) -> Self {
        Self::new(x, y)
    }
}

/// A 3D unsigned integer vector
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct UVec3 {
    /// X component
    pub x: u32,
    /// Y component
    pub y: u32,
    /// Z component
    pub z: u32,
}

impl UVec3 {
    /// Zero vector
    pub const ZERO: Self = Self::new(0, 0, 0);
    /// One vector
    pub const ONE: Self = Self::new(1, 1, 1);
    /// X unit vector
    pub const X: Self = Self::new(1, 0, 0);
    /// Y unit vector
    pub const Y: Self = Self::new(0, 1, 0);
    /// Z unit vector
    pub const Z: Self = Self::new(0, 0, 1);

    /// Creates a new vector
    #[inline]
    pub const fn new(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }

    /// Creates a vector with all components set to the same value
    #[inline]
    pub const fn splat(v: u32) -> Self {
        Self::new(v, v, v)
    }

    /// Converts to a float vector
    #[inline]
    pub fn as_vec3(self) -> crate::vec::Vec3 {
        crate::vec::Vec3::new(self.x as f32, self.y as f32, self.z as f32)
    }

    /// Extends to UVec4
    #[inline]
    pub const fn extend(self, w: u32) -> UVec4 {
        UVec4::new(self.x, self.y, self.z, w)
    }

    /// Truncates to UVec2
    #[inline]
    pub const fn truncate(self) -> UVec2 {
        UVec2::new(self.x, self.y)
    }

    /// Returns XY components
    #[inline]
    pub const fn xy(self) -> UVec2 {
        self.truncate()
    }

    /// Component-wise minimum
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self::new(
            self.x.min(other.x),
            self.y.min(other.y),
            self.z.min(other.z),
        )
    }

    /// Component-wise maximum
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self::new(
            self.x.max(other.x),
            self.y.max(other.y),
            self.z.max(other.z),
        )
    }

    /// Component-wise clamp
    #[inline]
    pub fn clamp(self, min: Self, max: Self) -> Self {
        self.max(min).min(max)
    }

    /// Dot product
    #[inline]
    pub fn dot(self, other: Self) -> u32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Length squared
    #[inline]
    pub fn length_squared(self) -> u32 {
        self.dot(self)
    }

    /// Converts to signed integer vector
    #[inline]
    pub fn as_ivec3(self) -> IVec3 {
        IVec3::new(self.x as i32, self.y as i32, self.z as i32)
    }
}

impl Add for UVec3 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl AddAssign for UVec3 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for UVec3 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl SubAssign for UVec3 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<u32> for UVec3 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: u32) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl MulAssign<u32> for UVec3 {
    #[inline]
    fn mul_assign(&mut self, rhs: u32) {
        *self = *self * rhs;
    }
}

impl Div<u32> for UVec3 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: u32) -> Self {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

impl DivAssign<u32> for UVec3 {
    #[inline]
    fn div_assign(&mut self, rhs: u32) {
        *self = *self / rhs;
    }
}

impl From<[u32; 3]> for UVec3 {
    #[inline]
    fn from(arr: [u32; 3]) -> Self {
        Self::new(arr[0], arr[1], arr[2])
    }
}

impl From<UVec3> for [u32; 3] {
    #[inline]
    fn from(v: UVec3) -> Self {
        [v.x, v.y, v.z]
    }
}

impl From<(u32, u32, u32)> for UVec3 {
    #[inline]
    fn from((x, y, z): (u32, u32, u32)) -> Self {
        Self::new(x, y, z)
    }
}

/// A 4D unsigned integer vector
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct UVec4 {
    /// X component
    pub x: u32,
    /// Y component
    pub y: u32,
    /// Z component
    pub z: u32,
    /// W component
    pub w: u32,
}

impl UVec4 {
    /// Zero vector
    pub const ZERO: Self = Self::new(0, 0, 0, 0);
    /// One vector
    pub const ONE: Self = Self::new(1, 1, 1, 1);
    /// X unit vector
    pub const X: Self = Self::new(1, 0, 0, 0);
    /// Y unit vector
    pub const Y: Self = Self::new(0, 1, 0, 0);
    /// Z unit vector
    pub const Z: Self = Self::new(0, 0, 1, 0);
    /// W unit vector
    pub const W: Self = Self::new(0, 0, 0, 1);

    /// Creates a new vector
    #[inline]
    pub const fn new(x: u32, y: u32, z: u32, w: u32) -> Self {
        Self { x, y, z, w }
    }

    /// Creates a vector with all components set to the same value
    #[inline]
    pub const fn splat(v: u32) -> Self {
        Self::new(v, v, v, v)
    }

    /// Converts to a float vector
    #[inline]
    pub fn as_vec4(self) -> crate::vec::Vec4 {
        crate::vec::Vec4::new(self.x as f32, self.y as f32, self.z as f32, self.w as f32)
    }

    /// Truncates to UVec3
    #[inline]
    pub const fn truncate(self) -> UVec3 {
        UVec3::new(self.x, self.y, self.z)
    }

    /// Returns XYZ components
    #[inline]
    pub const fn xyz(self) -> UVec3 {
        self.truncate()
    }

    /// Returns XY components
    #[inline]
    pub const fn xy(self) -> UVec2 {
        UVec2::new(self.x, self.y)
    }

    /// Component-wise minimum
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self::new(
            self.x.min(other.x),
            self.y.min(other.y),
            self.z.min(other.z),
            self.w.min(other.w),
        )
    }

    /// Component-wise maximum
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self::new(
            self.x.max(other.x),
            self.y.max(other.y),
            self.z.max(other.z),
            self.w.max(other.w),
        )
    }

    /// Component-wise clamp
    #[inline]
    pub fn clamp(self, min: Self, max: Self) -> Self {
        self.max(min).min(max)
    }

    /// Dot product
    #[inline]
    pub fn dot(self, other: Self) -> u32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }

    /// Length squared
    #[inline]
    pub fn length_squared(self) -> u32 {
        self.dot(self)
    }

    /// Converts to signed integer vector
    #[inline]
    pub fn as_ivec4(self) -> IVec4 {
        IVec4::new(self.x as i32, self.y as i32, self.z as i32, self.w as i32)
    }
}

impl Add for UVec4 {
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

impl AddAssign for UVec4 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for UVec4 {
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

impl SubAssign for UVec4 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<u32> for UVec4 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: u32) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs, self.w * rhs)
    }
}

impl MulAssign<u32> for UVec4 {
    #[inline]
    fn mul_assign(&mut self, rhs: u32) {
        *self = *self * rhs;
    }
}

impl Div<u32> for UVec4 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: u32) -> Self {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs, self.w / rhs)
    }
}

impl DivAssign<u32> for UVec4 {
    #[inline]
    fn div_assign(&mut self, rhs: u32) {
        *self = *self / rhs;
    }
}

impl From<[u32; 4]> for UVec4 {
    #[inline]
    fn from(arr: [u32; 4]) -> Self {
        Self::new(arr[0], arr[1], arr[2], arr[3])
    }
}

impl From<UVec4> for [u32; 4] {
    #[inline]
    fn from(v: UVec4) -> Self {
        [v.x, v.y, v.z, v.w]
    }
}

impl From<(u32, u32, u32, u32)> for UVec4 {
    #[inline]
    fn from((x, y, z, w): (u32, u32, u32, u32)) -> Self {
        Self::new(x, y, z, w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ivec3_operations() {
        let a = IVec3::new(1, 2, 3);
        let b = IVec3::new(4, 5, 6);
        let c = a + b;
        assert_eq!(c, IVec3::new(5, 7, 9));
    }

    #[test]
    fn test_uvec3_operations() {
        let a = UVec3::new(1, 2, 3);
        let b = UVec3::new(4, 5, 6);
        let c = a + b;
        assert_eq!(c, UVec3::new(5, 7, 9));
    }

    #[test]
    fn test_ivec_conversion() {
        let i = IVec3::new(1, 2, 3);
        let f = i.as_vec3();
        assert!((f.x - 1.0).abs() < 1e-5);
    }
}
