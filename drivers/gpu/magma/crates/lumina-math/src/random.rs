//! Random number generation for graphics
//!
//! This module provides random number generators suitable for GPU use.

use crate::Vec2;
use crate::Vec3;
use crate::Vec4;

/// PCG random number generator (32-bit)
#[derive(Clone, Copy, Debug)]
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    /// Default increment (must be odd)
    const DEFAULT_INC: u64 = 1442695040888963407;

    /// Creates a new PCG with the given seed
    pub const fn new(seed: u64) -> Self {
        let mut rng = Self {
            state: 0,
            inc: Self::DEFAULT_INC,
        };
        rng.state = seed.wrapping_add(rng.inc);
        rng.step();
        rng
    }

    /// Creates a PCG with seed and stream
    pub const fn with_stream(seed: u64, stream: u64) -> Self {
        let mut rng = Self {
            state: 0,
            inc: (stream << 1) | 1,
        };
        rng.state = seed.wrapping_add(rng.inc);
        rng.step();
        rng
    }

    /// Steps the RNG state
    const fn step(&mut self) {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(self.inc);
    }

    /// Generates the next u32
    pub fn next_u32(&mut self) -> u32 {
        let old_state = self.state;
        self.step();

        let xorshifted = (((old_state >> 18) ^ old_state) >> 27) as u32;
        let rot = (old_state >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Generates a u32 in range [0, bound)
    pub fn next_u32_bounded(&mut self, bound: u32) -> u32 {
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let r = self.next_u32();
            if r >= threshold {
                return r % bound;
            }
        }
    }

    /// Generates a float in [0, 1)
    pub fn next_f32(&mut self) -> f32 {
        // Use upper 24 bits for better distribution
        (self.next_u32() >> 8) as f32 * (1.0 / 16777216.0)
    }

    /// Generates a float in [0, 1]
    pub fn next_f32_inclusive(&mut self) -> f32 {
        self.next_u32() as f32 * (1.0 / 4294967295.0)
    }

    /// Generates a float in [-1, 1]
    pub fn next_f32_signed(&mut self) -> f32 {
        self.next_f32() * 2.0 - 1.0
    }

    /// Generates a float in [min, max)
    pub fn next_f32_range(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }

    /// Generates a Vec2 with components in [0, 1)
    pub fn next_vec2(&mut self) -> Vec2 {
        Vec2::new(self.next_f32(), self.next_f32())
    }

    /// Generates a Vec3 with components in [0, 1)
    pub fn next_vec3(&mut self) -> Vec3 {
        Vec3::new(self.next_f32(), self.next_f32(), self.next_f32())
    }

    /// Generates a Vec4 with components in [0, 1)
    pub fn next_vec4(&mut self) -> Vec4 {
        Vec4::new(
            self.next_f32(),
            self.next_f32(),
            self.next_f32(),
            self.next_f32(),
        )
    }

    /// Generates a point uniformly distributed on a unit sphere
    pub fn next_unit_sphere(&mut self) -> Vec3 {
        loop {
            let v = Vec3::new(
                self.next_f32_signed(),
                self.next_f32_signed(),
                self.next_f32_signed(),
            );
            let len_sq = v.length_squared();
            if len_sq > 0.0001 && len_sq <= 1.0 {
                return v / len_sq.sqrt();
            }
        }
    }

    /// Generates a point uniformly distributed inside a unit sphere
    pub fn next_in_unit_sphere(&mut self) -> Vec3 {
        loop {
            let v = Vec3::new(
                self.next_f32_signed(),
                self.next_f32_signed(),
                self.next_f32_signed(),
            );
            if v.length_squared() <= 1.0 {
                return v;
            }
        }
    }

    /// Generates a point uniformly distributed on a unit disk
    pub fn next_unit_disk(&mut self) -> Vec2 {
        loop {
            let v = Vec2::new(self.next_f32_signed(), self.next_f32_signed());
            if v.length_squared() <= 1.0 {
                return v;
            }
        }
    }

    /// Generates a point uniformly distributed in a unit circle
    pub fn next_in_unit_circle(&mut self) -> Vec2 {
        let theta = self.next_f32() * core::f32::consts::TAU;
        let r = self.next_f32().sqrt();
        Vec2::new(r * theta.cos(), r * theta.sin())
    }

    /// Generates a cosine-weighted hemisphere direction
    pub fn next_cosine_hemisphere(&mut self) -> Vec3 {
        let r1 = self.next_f32();
        let r2 = self.next_f32();
        let z = (1.0 - r2).sqrt();
        let phi = core::f32::consts::TAU * r1;
        let sin_theta = r2.sqrt();
        Vec3::new(phi.cos() * sin_theta, phi.sin() * sin_theta, z)
    }

    /// Generates a uniform hemisphere direction
    pub fn next_uniform_hemisphere(&mut self) -> Vec3 {
        let r1 = self.next_f32();
        let r2 = self.next_f32();
        let sin_theta = (1.0 - r1 * r1).sqrt();
        let phi = core::f32::consts::TAU * r2;
        Vec3::new(sin_theta * phi.cos(), sin_theta * phi.sin(), r1)
    }

    /// Shuffles a slice in place
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = self.next_u32_bounded((i + 1) as u32) as usize;
            slice.swap(i, j);
        }
    }

    /// Returns true with probability p
    pub fn next_bool(&mut self, p: f32) -> bool {
        self.next_f32() < p
    }
}

impl Default for Pcg32 {
    fn default() -> Self {
        Self::new(0x853c49e6748fea9b)
    }
}

/// Xorshift128+ random number generator
#[derive(Clone, Copy, Debug)]
pub struct Xorshift128Plus {
    s0: u64,
    s1: u64,
}

impl Xorshift128Plus {
    /// Creates a new generator with the given seed
    pub const fn new(seed: u64) -> Self {
        // Use splitmix64 to initialize state
        let z0 = seed.wrapping_add(0x9e3779b97f4a7c15);
        let z1 = (z0 ^ (z0 >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        let z2 = (z1 ^ (z1 >> 27)).wrapping_mul(0x94d049bb133111eb);
        let s0 = z2 ^ (z2 >> 31);

        let z3 = s0.wrapping_add(0x9e3779b97f4a7c15);
        let z4 = (z3 ^ (z3 >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        let z5 = (z4 ^ (z4 >> 27)).wrapping_mul(0x94d049bb133111eb);
        let s1 = z5 ^ (z5 >> 31);

        Self { s0, s1 }
    }

    /// Generates the next u64
    pub fn next_u64(&mut self) -> u64 {
        let mut s1 = self.s0;
        let s0 = self.s1;
        let result = s0.wrapping_add(s1);
        self.s0 = s0;
        s1 ^= s1 << 23;
        self.s1 = s1 ^ s0 ^ (s1 >> 18) ^ (s0 >> 5);
        result
    }

    /// Generates the next u32
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Generates a float in [0, 1)
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 * (1.0 / 16777216.0)
    }

    /// Generates a double in [0, 1)
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / 9007199254740992.0)
    }
}

impl Default for Xorshift128Plus {
    fn default() -> Self {
        Self::new(0x853c49e6748fea9b)
    }
}

/// Hash-based random (for GPU use)
pub mod hash {
    use super::*;

    /// Wang hash
    pub const fn wang_hash(mut key: u32) -> u32 {
        key = (key ^ 61) ^ (key >> 16);
        key = key.wrapping_add(key << 3);
        key = key ^ (key >> 4);
        key = key.wrapping_mul(0x27d4eb2d);
        key = key ^ (key >> 15);
        key
    }

    /// XXHash32 single round
    pub const fn xxhash32(seed: u32, input: u32) -> u32 {
        const PRIME1: u32 = 0x9E3779B1;
        const PRIME2: u32 = 0x85EBCA77;
        const PRIME3: u32 = 0xC2B2AE3D;

        let mut h = seed.wrapping_add(PRIME1);
        h = h.wrapping_add(input.wrapping_mul(PRIME2));
        h = h.rotate_left(13);
        h = h.wrapping_mul(PRIME3);
        h ^= h >> 15;
        h = h.wrapping_mul(PRIME2);
        h ^= h >> 13;
        h = h.wrapping_mul(PRIME3);
        h ^= h >> 16;
        h
    }

    /// PCG one-off hash
    pub fn pcg_hash(input: u32) -> u32 {
        let state = input.wrapping_mul(747796405).wrapping_add(2891336453);
        let word = ((state >> ((state >> 28).wrapping_add(4))) ^ state).wrapping_mul(277803737);
        (word >> 22) ^ word
    }

    /// Converts hash to float [0, 1)
    pub fn hash_to_float(hash: u32) -> f32 {
        (hash >> 8) as f32 * (1.0 / 16777216.0)
    }

    /// 2D hash for textures
    pub fn hash2d(x: u32, y: u32, seed: u32) -> u32 {
        let h = wang_hash(x.wrapping_add(seed));
        wang_hash(y.wrapping_add(h))
    }

    /// 3D hash
    pub fn hash3d(x: u32, y: u32, z: u32, seed: u32) -> u32 {
        let h = wang_hash(x.wrapping_add(seed));
        let h = wang_hash(y.wrapping_add(h));
        wang_hash(z.wrapping_add(h))
    }

    /// Random Vec2 from seed
    pub fn random_vec2(seed: u32) -> Vec2 {
        let h1 = wang_hash(seed);
        let h2 = wang_hash(h1);
        Vec2::new(hash_to_float(h1), hash_to_float(h2))
    }

    /// Random Vec3 from seed
    pub fn random_vec3(seed: u32) -> Vec3 {
        let h1 = wang_hash(seed);
        let h2 = wang_hash(h1);
        let h3 = wang_hash(h2);
        Vec3::new(hash_to_float(h1), hash_to_float(h2), hash_to_float(h3))
    }

    /// Random unit sphere point from seed
    pub fn random_unit_sphere(seed: u32) -> Vec3 {
        let h1 = wang_hash(seed);
        let h2 = wang_hash(h1);

        let z = hash_to_float(h1) * 2.0 - 1.0;
        let phi = hash_to_float(h2) * core::f32::consts::TAU;
        let r = (1.0 - z * z).sqrt();

        Vec3::new(r * phi.cos(), r * phi.sin(), z)
    }
}

/// Low-discrepancy sequences
pub mod sequence {
    use super::*;

    /// Halton sequence value
    pub fn halton(index: u32, base: u32) -> f32 {
        let mut result = 0.0f32;
        let mut f = 1.0 / base as f32;
        let mut i = index;

        while i > 0 {
            result += f * (i % base) as f32;
            i /= base;
            f /= base as f32;
        }

        result
    }

    /// Halton 2D point
    pub fn halton_2d(index: u32) -> Vec2 {
        Vec2::new(halton(index, 2), halton(index, 3))
    }

    /// Halton 3D point
    pub fn halton_3d(index: u32) -> Vec3 {
        Vec3::new(halton(index, 2), halton(index, 3), halton(index, 5))
    }

    /// Van der Corput sequence (base 2)
    pub fn van_der_corput(mut bits: u32) -> f32 {
        bits = (bits << 16) | (bits >> 16);
        bits = ((bits & 0x55555555) << 1) | ((bits & 0xAAAAAAAA) >> 1);
        bits = ((bits & 0x33333333) << 2) | ((bits & 0xCCCCCCCC) >> 2);
        bits = ((bits & 0x0F0F0F0F) << 4) | ((bits & 0xF0F0F0F0) >> 4);
        bits = ((bits & 0x00FF00FF) << 8) | ((bits & 0xFF00FF00) >> 8);
        bits as f32 * 2.3283064365386963e-10
    }

    /// Hammersley 2D point set
    pub fn hammersley_2d(i: u32, n: u32) -> Vec2 {
        Vec2::new(i as f32 / n as f32, van_der_corput(i))
    }

    /// Golden ratio sequence (1D)
    pub fn golden_sequence(index: u32) -> f32 {
        const PHI: f32 = 1.618033988749895;
        let x = index as f32 * (1.0 / PHI);
        x - x.floor()
    }

    /// R2 sequence (2D low-discrepancy)
    pub fn r2_sequence(index: u32) -> Vec2 {
        const G: f32 = 1.32471795724474602596;
        let a1 = 1.0 / G;
        let a2 = 1.0 / (G * G);

        let x = (0.5 + a1 * index as f32) % 1.0;
        let y = (0.5 + a2 * index as f32) % 1.0;

        Vec2::new(x, y)
    }

    /// Blue noise sample (requires offset texture)
    pub fn blue_noise_offset(pixel: (u32, u32), frame: u32, texture_size: u32) -> Vec2 {
        let x = (pixel.0 + frame * 37) % texture_size;
        let y = (pixel.1 + frame * 59) % texture_size;
        // In practice, you'd sample a blue noise texture here
        halton_2d(x + y * texture_size)
    }

    /// Poisson disk sample candidates
    pub fn poisson_disk_candidate(center: Vec2, min_dist: f32, rng: &mut super::Pcg32) -> Vec2 {
        let angle = rng.next_f32() * core::f32::consts::TAU;
        let radius = min_dist + rng.next_f32() * min_dist;
        center + Vec2::new(angle.cos(), angle.sin()) * radius
    }
}

/// Stratified sampling
pub mod stratified {
    use super::*;

    /// Generates stratified 1D samples
    pub fn samples_1d(count: u32, rng: &mut super::Pcg32) -> alloc::vec::Vec<f32> {
        let inv = 1.0 / count as f32;
        (0..count)
            .map(|i| (i as f32 + rng.next_f32()) * inv)
            .collect()
    }

    /// Generates stratified 2D samples
    pub fn samples_2d(nx: u32, ny: u32, rng: &mut super::Pcg32) -> alloc::vec::Vec<Vec2> {
        let inv_x = 1.0 / nx as f32;
        let inv_y = 1.0 / ny as f32;
        let mut samples = alloc::vec::Vec::with_capacity((nx * ny) as usize);

        for y in 0..ny {
            for x in 0..nx {
                let px = (x as f32 + rng.next_f32()) * inv_x;
                let py = (y as f32 + rng.next_f32()) * inv_y;
                samples.push(Vec2::new(px, py));
            }
        }

        samples
    }

    /// Generates jittered samples on a disk
    pub fn disk_samples(count: u32, rng: &mut super::Pcg32) -> alloc::vec::Vec<Vec2> {
        let mut samples = alloc::vec::Vec::with_capacity(count as usize);
        let rings = (count as f32).sqrt() as u32;

        for ring in 0..rings {
            let r0 = ring as f32 / rings as f32;
            let r1 = (ring + 1) as f32 / rings as f32;
            let samples_in_ring = if ring == 0 { 1 } else { ring * 6 };

            for i in 0..samples_in_ring {
                let theta0 = i as f32 / samples_in_ring as f32 * core::f32::consts::TAU;
                let theta1 = (i + 1) as f32 / samples_in_ring as f32 * core::f32::consts::TAU;

                let r = (r0 * r0 + rng.next_f32() * (r1 * r1 - r0 * r0)).sqrt();
                let theta = theta0 + rng.next_f32() * (theta1 - theta0);

                samples.push(Vec2::new(r * theta.cos(), r * theta.sin()));
            }
        }

        samples
    }
}

extern crate alloc;
