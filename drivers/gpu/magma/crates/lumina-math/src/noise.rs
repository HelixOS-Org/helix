//! Noise generation functions
//!
//! This module provides various noise functions for procedural generation.

use crate::vec::{Vec2, Vec3, Vec4};

/// Permutation table for noise functions
const PERM: [u8; 256] = [
    151, 160, 137, 91, 90, 15, 131, 13, 201, 95, 96, 53, 194, 233, 7, 225, 140, 36, 103, 30, 69,
    142, 8, 99, 37, 240, 21, 10, 23, 190, 6, 148, 247, 120, 234, 75, 0, 26, 197, 62, 94, 252, 219,
    203, 117, 35, 11, 32, 57, 177, 33, 88, 237, 149, 56, 87, 174, 20, 125, 136, 171, 168, 68, 175,
    74, 165, 71, 134, 139, 48, 27, 166, 77, 146, 158, 231, 83, 111, 229, 122, 60, 211, 133, 230,
    220, 105, 92, 41, 55, 46, 245, 40, 244, 102, 143, 54, 65, 25, 63, 161, 1, 216, 80, 73, 209, 76,
    132, 187, 208, 89, 18, 169, 200, 196, 135, 130, 116, 188, 159, 86, 164, 100, 109, 198, 173,
    186, 3, 64, 52, 217, 226, 250, 124, 123, 5, 202, 38, 147, 118, 126, 255, 82, 85, 212, 207, 206,
    59, 227, 47, 16, 58, 17, 182, 189, 28, 42, 223, 183, 170, 213, 119, 248, 152, 2, 44, 154, 163,
    70, 221, 153, 101, 155, 167, 43, 172, 9, 129, 22, 39, 253, 19, 98, 108, 110, 79, 113, 224, 232,
    178, 185, 112, 104, 218, 246, 97, 228, 251, 34, 242, 193, 238, 210, 144, 12, 191, 179, 162,
    241, 81, 51, 145, 235, 249, 14, 239, 107, 49, 192, 214, 31, 181, 199, 106, 157, 184, 84, 204,
    176, 115, 121, 50, 45, 127, 4, 150, 254, 138, 236, 205, 93, 222, 114, 67, 29, 24, 72, 243, 141,
    128, 195, 78, 66, 215, 61, 156, 180,
];

/// Gets a value from the permutation table
#[inline]
fn perm(i: i32) -> i32 {
    PERM[(i & 255) as usize] as i32
}

/// Gradient vectors for 2D noise
const GRAD2: [[f32; 2]; 8] = [
    [1.0, 1.0],
    [-1.0, 1.0],
    [1.0, -1.0],
    [-1.0, -1.0],
    [1.0, 0.0],
    [-1.0, 0.0],
    [0.0, 1.0],
    [0.0, -1.0],
];

/// Gradient vectors for 3D noise
const GRAD3: [[f32; 3]; 12] = [
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [1.0, -1.0, 0.0],
    [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0],
    [-1.0, 0.0, 1.0],
    [1.0, 0.0, -1.0],
    [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0],
    [0.0, -1.0, 1.0],
    [0.0, 1.0, -1.0],
    [0.0, -1.0, -1.0],
];

/// 2D gradient dot product
#[inline]
fn grad2(hash: i32, x: f32, y: f32) -> f32 {
    let g = &GRAD2[(hash & 7) as usize];
    g[0] * x + g[1] * y
}

/// 3D gradient dot product
#[inline]
fn grad3(hash: i32, x: f32, y: f32, z: f32) -> f32 {
    let g = &GRAD3[(hash % 12) as usize];
    g[0] * x + g[1] * y + g[2] * z
}

/// Fade function for smooth interpolation
#[inline]
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Perlin noise implementation
pub mod perlin {
    use super::*;

    /// 2D Perlin noise
    /// Returns a value in the range [-1, 1]
    #[inline]
    pub fn noise2(x: f32, y: f32) -> f32 {
        // Integer coordinates
        let xi = x.floor() as i32;
        let yi = y.floor() as i32;

        // Fractional coordinates
        let xf = x - xi as f32;
        let yf = y - yi as f32;

        // Fade curves
        let u = fade(xf);
        let v = fade(yf);

        // Hash coordinates
        let aa = perm(perm(xi) + yi);
        let ab = perm(perm(xi) + yi + 1);
        let ba = perm(perm(xi + 1) + yi);
        let bb = perm(perm(xi + 1) + yi + 1);

        // Gradient values
        let x1 = lerp(grad2(aa, xf, yf), grad2(ba, xf - 1.0, yf), u);
        let x2 = lerp(grad2(ab, xf, yf - 1.0), grad2(bb, xf - 1.0, yf - 1.0), u);

        lerp(x1, x2, v)
    }

    /// 3D Perlin noise
    /// Returns a value in the range [-1, 1]
    #[inline]
    pub fn noise3(x: f32, y: f32, z: f32) -> f32 {
        // Integer coordinates
        let xi = x.floor() as i32;
        let yi = y.floor() as i32;
        let zi = z.floor() as i32;

        // Fractional coordinates
        let xf = x - xi as f32;
        let yf = y - yi as f32;
        let zf = z - zi as f32;

        // Fade curves
        let u = fade(xf);
        let v = fade(yf);
        let w = fade(zf);

        // Hash coordinates
        let aaa = perm(perm(perm(xi) + yi) + zi);
        let aba = perm(perm(perm(xi) + yi + 1) + zi);
        let aab = perm(perm(perm(xi) + yi) + zi + 1);
        let abb = perm(perm(perm(xi) + yi + 1) + zi + 1);
        let baa = perm(perm(perm(xi + 1) + yi) + zi);
        let bba = perm(perm(perm(xi + 1) + yi + 1) + zi);
        let bab = perm(perm(perm(xi + 1) + yi) + zi + 1);
        let bbb = perm(perm(perm(xi + 1) + yi + 1) + zi + 1);

        // Gradient values
        let x1 = lerp(grad3(aaa, xf, yf, zf), grad3(baa, xf - 1.0, yf, zf), u);
        let x2 = lerp(
            grad3(aba, xf, yf - 1.0, zf),
            grad3(bba, xf - 1.0, yf - 1.0, zf),
            u,
        );
        let y1 = lerp(x1, x2, v);

        let x1 = lerp(
            grad3(aab, xf, yf, zf - 1.0),
            grad3(bab, xf - 1.0, yf, zf - 1.0),
            u,
        );
        let x2 = lerp(
            grad3(abb, xf, yf - 1.0, zf - 1.0),
            grad3(bbb, xf - 1.0, yf - 1.0, zf - 1.0),
            u,
        );
        let y2 = lerp(x1, x2, v);

        lerp(y1, y2, w)
    }

    /// 2D Perlin noise from Vec2
    #[inline]
    pub fn noise2_vec(p: Vec2) -> f32 {
        noise2(p.x, p.y)
    }

    /// 3D Perlin noise from Vec3
    #[inline]
    pub fn noise3_vec(p: Vec3) -> f32 {
        noise3(p.x, p.y, p.z)
    }
}

/// Linear interpolation helper
#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

/// Fractal Brownian Motion (FBM) noise
pub mod fbm {
    use super::*;

    /// 2D FBM noise using Perlin noise
    pub fn noise2(x: f32, y: f32, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
        let mut total = 0.0;
        let mut frequency = 1.0;
        let mut amplitude = 1.0;
        let mut max_value = 0.0;

        for _ in 0..octaves {
            total += perlin::noise2(x * frequency, y * frequency) * amplitude;
            max_value += amplitude;
            frequency *= lacunarity;
            amplitude *= gain;
        }

        total / max_value
    }

    /// 3D FBM noise using Perlin noise
    pub fn noise3(x: f32, y: f32, z: f32, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
        let mut total = 0.0;
        let mut frequency = 1.0;
        let mut amplitude = 1.0;
        let mut max_value = 0.0;

        for _ in 0..octaves {
            total += perlin::noise3(x * frequency, y * frequency, z * frequency) * amplitude;
            max_value += amplitude;
            frequency *= lacunarity;
            amplitude *= gain;
        }

        total / max_value
    }

    /// 2D FBM with default parameters
    #[inline]
    pub fn noise2_default(x: f32, y: f32) -> f32 {
        noise2(x, y, 6, 2.0, 0.5)
    }

    /// 3D FBM with default parameters
    #[inline]
    pub fn noise3_default(x: f32, y: f32, z: f32) -> f32 {
        noise3(x, y, z, 6, 2.0, 0.5)
    }
}

/// Simplex noise implementation
pub mod simplex {
    use super::*;

    const F2: f32 = 0.5 * (1.732050808 - 1.0); // (sqrt(3) - 1) / 2
    const G2: f32 = (3.0 - 1.732050808) / 6.0; // (3 - sqrt(3)) / 6
    const F3: f32 = 1.0 / 3.0;
    const G3: f32 = 1.0 / 6.0;

    /// 2D Simplex noise
    /// Returns a value in approximately [-1, 1]
    pub fn noise2(x: f32, y: f32) -> f32 {
        // Skew input space
        let s = (x + y) * F2;
        let i = (x + s).floor() as i32;
        let j = (y + s).floor() as i32;

        // Unskew
        let t = (i + j) as f32 * G2;
        let x0 = x - (i as f32 - t);
        let y0 = y - (j as f32 - t);

        // Determine which simplex
        let (i1, j1) = if x0 > y0 { (1, 0) } else { (0, 1) };

        // Offsets for corners
        let x1 = x0 - i1 as f32 + G2;
        let y1 = y0 - j1 as f32 + G2;
        let x2 = x0 - 1.0 + 2.0 * G2;
        let y2 = y0 - 1.0 + 2.0 * G2;

        // Gradient indices
        let gi0 = perm(i + perm(j)) % 8;
        let gi1 = perm(i + i1 + perm(j + j1)) % 8;
        let gi2 = perm(i + 1 + perm(j + 1)) % 8;

        // Contributions from corners
        let n0 = contrib(gi0, x0, y0);
        let n1 = contrib(gi1, x1, y1);
        let n2 = contrib(gi2, x2, y2);

        // Scale to [-1, 1]
        70.0 * (n0 + n1 + n2)
    }

    #[inline]
    fn contrib(gi: i32, x: f32, y: f32) -> f32 {
        let t = 0.5 - x * x - y * y;
        if t < 0.0 {
            0.0
        } else {
            let t = t * t;
            t * t * grad2(gi, x, y)
        }
    }

    /// 3D Simplex noise
    /// Returns a value in approximately [-1, 1]
    pub fn noise3(x: f32, y: f32, z: f32) -> f32 {
        // Skew input space
        let s = (x + y + z) * F3;
        let i = (x + s).floor() as i32;
        let j = (y + s).floor() as i32;
        let k = (z + s).floor() as i32;

        // Unskew
        let t = (i + j + k) as f32 * G3;
        let x0 = x - (i as f32 - t);
        let y0 = y - (j as f32 - t);
        let z0 = z - (k as f32 - t);

        // Determine simplex
        let (i1, j1, k1, i2, j2, k2) = if x0 >= y0 {
            if y0 >= z0 {
                (1, 0, 0, 1, 1, 0)
            } else if x0 >= z0 {
                (1, 0, 0, 1, 0, 1)
            } else {
                (0, 0, 1, 1, 0, 1)
            }
        } else if y0 < z0 {
            (0, 0, 1, 0, 1, 1)
        } else if x0 < z0 {
            (0, 1, 0, 0, 1, 1)
        } else {
            (0, 1, 0, 1, 1, 0)
        };

        // Offsets
        let x1 = x0 - i1 as f32 + G3;
        let y1 = y0 - j1 as f32 + G3;
        let z1 = z0 - k1 as f32 + G3;
        let x2 = x0 - i2 as f32 + 2.0 * G3;
        let y2 = y0 - j2 as f32 + 2.0 * G3;
        let z2 = z0 - k2 as f32 + 2.0 * G3;
        let x3 = x0 - 1.0 + 3.0 * G3;
        let y3 = y0 - 1.0 + 3.0 * G3;
        let z3 = z0 - 1.0 + 3.0 * G3;

        // Gradient indices
        let gi0 = perm(i + perm(j + perm(k))) % 12;
        let gi1 = perm(i + i1 + perm(j + j1 + perm(k + k1))) % 12;
        let gi2 = perm(i + i2 + perm(j + j2 + perm(k + k2))) % 12;
        let gi3 = perm(i + 1 + perm(j + 1 + perm(k + 1))) % 12;

        // Contributions
        let n0 = contrib3(gi0, x0, y0, z0);
        let n1 = contrib3(gi1, x1, y1, z1);
        let n2 = contrib3(gi2, x2, y2, z2);
        let n3 = contrib3(gi3, x3, y3, z3);

        // Scale to [-1, 1]
        32.0 * (n0 + n1 + n2 + n3)
    }

    #[inline]
    fn contrib3(gi: i32, x: f32, y: f32, z: f32) -> f32 {
        let t = 0.6 - x * x - y * y - z * z;
        if t < 0.0 {
            0.0
        } else {
            let t = t * t;
            t * t * grad3(gi, x, y, z)
        }
    }

    /// 2D Simplex noise from Vec2
    #[inline]
    pub fn noise2_vec(p: Vec2) -> f32 {
        noise2(p.x, p.y)
    }

    /// 3D Simplex noise from Vec3
    #[inline]
    pub fn noise3_vec(p: Vec3) -> f32 {
        noise3(p.x, p.y, p.z)
    }
}

/// Worley (cellular) noise implementation
pub mod worley {
    use super::*;

    /// 2D Worley noise - returns distance to nearest point
    pub fn noise2(x: f32, y: f32) -> f32 {
        let xi = x.floor() as i32;
        let yi = y.floor() as i32;
        let xf = x - xi as f32;
        let yf = y - yi as f32;

        let mut min_dist = f32::MAX;

        // Check surrounding cells
        for dx in -1..=1 {
            for dy in -1..=1 {
                let cell_x = xi + dx;
                let cell_y = yi + dy;

                // Pseudo-random point in cell
                let hash = perm(cell_x + perm(cell_y));
                let px = dx as f32 + hash_to_float(hash) - xf;
                let py = dy as f32 + hash_to_float(hash * 16807) - yf;

                let dist = px * px + py * py;
                min_dist = min_dist.min(dist);
            }
        }

        min_dist.sqrt()
    }

    /// 3D Worley noise - returns distance to nearest point
    pub fn noise3(x: f32, y: f32, z: f32) -> f32 {
        let xi = x.floor() as i32;
        let yi = y.floor() as i32;
        let zi = z.floor() as i32;
        let xf = x - xi as f32;
        let yf = y - yi as f32;
        let zf = z - zi as f32;

        let mut min_dist = f32::MAX;

        // Check surrounding cells
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let cell_x = xi + dx;
                    let cell_y = yi + dy;
                    let cell_z = zi + dz;

                    // Pseudo-random point in cell
                    let hash = perm(cell_x + perm(cell_y + perm(cell_z)));
                    let px = dx as f32 + hash_to_float(hash) - xf;
                    let py = dy as f32 + hash_to_float(hash * 16807) - yf;
                    let pz = dz as f32 + hash_to_float(hash * 48271) - zf;

                    let dist = px * px + py * py + pz * pz;
                    min_dist = min_dist.min(dist);
                }
            }
        }

        min_dist.sqrt()
    }

    /// 2D Worley noise with second nearest distance
    pub fn noise2_f2(x: f32, y: f32) -> (f32, f32) {
        let xi = x.floor() as i32;
        let yi = y.floor() as i32;
        let xf = x - xi as f32;
        let yf = y - yi as f32;

        let mut d1 = f32::MAX;
        let mut d2 = f32::MAX;

        for dx in -1..=1 {
            for dy in -1..=1 {
                let cell_x = xi + dx;
                let cell_y = yi + dy;

                let hash = perm(cell_x + perm(cell_y));
                let px = dx as f32 + hash_to_float(hash) - xf;
                let py = dy as f32 + hash_to_float(hash * 16807) - yf;

                let dist = px * px + py * py;
                if dist < d1 {
                    d2 = d1;
                    d1 = dist;
                } else if dist < d2 {
                    d2 = dist;
                }
            }
        }

        (d1.sqrt(), d2.sqrt())
    }

    #[inline]
    fn hash_to_float(hash: i32) -> f32 {
        (hash & 0x7FFFFFFF) as f32 / 2147483647.0
    }
}

/// Value noise implementation
pub mod value {
    use super::*;

    /// 2D value noise
    pub fn noise2(x: f32, y: f32) -> f32 {
        let xi = x.floor() as i32;
        let yi = y.floor() as i32;
        let xf = x - xi as f32;
        let yf = y - yi as f32;

        let u = fade(xf);
        let v = fade(yf);

        let c00 = hash_to_float(perm(xi + perm(yi)));
        let c10 = hash_to_float(perm(xi + 1 + perm(yi)));
        let c01 = hash_to_float(perm(xi + perm(yi + 1)));
        let c11 = hash_to_float(perm(xi + 1 + perm(yi + 1)));

        let x1 = lerp(c00, c10, u);
        let x2 = lerp(c01, c11, u);

        lerp(x1, x2, v) * 2.0 - 1.0
    }

    /// 3D value noise
    pub fn noise3(x: f32, y: f32, z: f32) -> f32 {
        let xi = x.floor() as i32;
        let yi = y.floor() as i32;
        let zi = z.floor() as i32;
        let xf = x - xi as f32;
        let yf = y - yi as f32;
        let zf = z - zi as f32;

        let u = fade(xf);
        let v = fade(yf);
        let w = fade(zf);

        let c000 = hash_to_float(perm(xi + perm(yi + perm(zi))));
        let c100 = hash_to_float(perm(xi + 1 + perm(yi + perm(zi))));
        let c010 = hash_to_float(perm(xi + perm(yi + 1 + perm(zi))));
        let c110 = hash_to_float(perm(xi + 1 + perm(yi + 1 + perm(zi))));
        let c001 = hash_to_float(perm(xi + perm(yi + perm(zi + 1))));
        let c101 = hash_to_float(perm(xi + 1 + perm(yi + perm(zi + 1))));
        let c011 = hash_to_float(perm(xi + perm(yi + 1 + perm(zi + 1))));
        let c111 = hash_to_float(perm(xi + 1 + perm(yi + 1 + perm(zi + 1))));

        let x1 = lerp(c000, c100, u);
        let x2 = lerp(c010, c110, u);
        let y1 = lerp(x1, x2, v);

        let x1 = lerp(c001, c101, u);
        let x2 = lerp(c011, c111, u);
        let y2 = lerp(x1, x2, v);

        lerp(y1, y2, w) * 2.0 - 1.0
    }

    #[inline]
    fn hash_to_float(hash: i32) -> f32 {
        (hash & 0xFF) as f32 / 255.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perlin2_range() {
        for i in 0..100 {
            let x = i as f32 * 0.1;
            let y = i as f32 * 0.17;
            let v = perlin::noise2(x, y);
            assert!(v >= -1.0 && v <= 1.0, "Perlin noise out of range: {}", v);
        }
    }

    #[test]
    fn test_perlin3_range() {
        for i in 0..100 {
            let x = i as f32 * 0.1;
            let y = i as f32 * 0.17;
            let z = i as f32 * 0.23;
            let v = perlin::noise3(x, y, z);
            assert!(v >= -1.0 && v <= 1.0, "Perlin noise out of range: {}", v);
        }
    }

    #[test]
    fn test_simplex2_deterministic() {
        let v1 = simplex::noise2(1.0, 2.0);
        let v2 = simplex::noise2(1.0, 2.0);
        assert!(
            (v1 - v2).abs() < 1e-10,
            "Simplex noise should be deterministic"
        );
    }

    #[test]
    fn test_worley2_positive() {
        for i in 0..100 {
            let x = i as f32 * 0.1;
            let y = i as f32 * 0.17;
            let v = worley::noise2(x, y);
            assert!(v >= 0.0, "Worley noise should be positive: {}", v);
        }
    }
}
