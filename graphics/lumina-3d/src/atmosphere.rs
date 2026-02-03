//! # Atmospheric Rendering
//!
//! Physical sky and atmosphere simulation.

use alloc::vec::Vec;

/// Atmosphere renderer
pub struct AtmosphereRenderer {
    params: AtmosphereParams,
    lut_transmittance: Vec<f32>,
    lut_scattering: Vec<f32>,
    lut_irradiance: Vec<f32>,
    needs_rebuild: bool,
}

impl AtmosphereRenderer {
    pub fn new(params: AtmosphereParams) -> Self {
        let mut renderer = Self {
            params,
            lut_transmittance: Vec::new(),
            lut_scattering: Vec::new(),
            lut_irradiance: Vec::new(),
            needs_rebuild: true,
        };
        renderer.rebuild_luts();
        renderer
    }

    /// Update atmosphere parameters
    pub fn set_params(&mut self, params: AtmosphereParams) {
        self.params = params;
        self.needs_rebuild = true;
    }

    /// Rebuild lookup tables
    pub fn rebuild_luts(&mut self) {
        if !self.needs_rebuild {
            return;
        }

        self.compute_transmittance_lut();
        self.compute_scattering_lut();
        self.compute_irradiance_lut();

        self.needs_rebuild = false;
    }

    fn compute_transmittance_lut(&mut self) {
        let width = 256;
        let height = 64;
        self.lut_transmittance = vec![0.0; width * height * 4];

        for y in 0..height {
            for x in 0..width {
                let cos_zenith = 2.0 * x as f32 / (width - 1) as f32 - 1.0;
                let altitude = y as f32 / (height - 1) as f32
                    * (self.params.atmosphere_height - self.params.ground_radius);

                let transmittance = self.compute_transmittance(altitude, cos_zenith);
                let idx = (y * width + x) * 4;
                self.lut_transmittance[idx] = transmittance[0];
                self.lut_transmittance[idx + 1] = transmittance[1];
                self.lut_transmittance[idx + 2] = transmittance[2];
                self.lut_transmittance[idx + 3] = 1.0;
            }
        }
    }

    fn compute_scattering_lut(&mut self) {
        let width = 256;
        let height = 128;
        let depth = 32;
        self.lut_scattering = vec![0.0; width * height * depth * 4];

        // Would compute multi-scattering LUT
    }

    fn compute_irradiance_lut(&mut self) {
        let width = 64;
        let height = 16;
        self.lut_irradiance = vec![0.0; width * height * 4];

        // Would compute ground irradiance LUT
    }

    fn compute_transmittance(&self, altitude: f32, cos_zenith: f32) -> [f32; 3] {
        let r = self.params.ground_radius + altitude;
        let mu = cos_zenith;

        // Ray-sphere intersection for atmosphere top
        let discriminant =
            r * r * (mu * mu - 1.0) + self.params.atmosphere_height * self.params.atmosphere_height;
        if discriminant < 0.0 {
            return [1.0, 1.0, 1.0];
        }

        let d = (-r * mu + discriminant.sqrt()).max(0.0);

        // Numerical integration along ray
        let steps = 32;
        let step_size = d / steps as f32;

        let mut optical_depth_r = 0.0f32;
        let mut optical_depth_m = 0.0f32;

        for i in 0..steps {
            let t = (i as f32 + 0.5) * step_size;
            let pos_height = ((r + t * mu).powi(2) + t.powi(2) * (1.0 - mu * mu)).sqrt()
                - self.params.ground_radius;

            let density_r = (-pos_height / self.params.rayleigh_scale_height).exp();
            let density_m = (-pos_height / self.params.mie_scale_height).exp();

            optical_depth_r += density_r * step_size;
            optical_depth_m += density_m * step_size;
        }

        let extinction_r = [
            self.params.rayleigh_scattering[0] * optical_depth_r,
            self.params.rayleigh_scattering[1] * optical_depth_r,
            self.params.rayleigh_scattering[2] * optical_depth_r,
        ];

        let extinction_m = self.params.mie_extinction * optical_depth_m;

        [
            (-(extinction_r[0] + extinction_m)).exp(),
            (-(extinction_r[1] + extinction_m)).exp(),
            (-(extinction_r[2] + extinction_m)).exp(),
        ]
    }

    /// Get sky color for a view direction
    pub fn get_sky_color(&self, view_dir: [f32; 3], sun_dir: [f32; 3]) -> [f32; 3] {
        let cos_sun = dot(view_dir, sun_dir);

        // Rayleigh phase function
        let phase_r = 3.0 / (16.0 * core::f32::consts::PI) * (1.0 + cos_sun * cos_sun);

        // Mie phase function (Henyey-Greenstein)
        let g = self.params.mie_g;
        let phase_m = 3.0 / (8.0 * core::f32::consts::PI)
            * ((1.0 - g * g) * (1.0 + cos_sun * cos_sun))
            / ((2.0 + g * g) * (1.0 + g * g - 2.0 * g * cos_sun).powf(1.5));

        // Would sample LUTs here
        let scattering = [0.3, 0.5, 0.8]; // Placeholder

        [
            scattering[0]
                * (phase_r * self.params.rayleigh_scattering[0]
                    + phase_m * self.params.mie_scattering),
            scattering[1]
                * (phase_r * self.params.rayleigh_scattering[1]
                    + phase_m * self.params.mie_scattering),
            scattering[2]
                * (phase_r * self.params.rayleigh_scattering[2]
                    + phase_m * self.params.mie_scattering),
        ]
    }
}

/// Atmosphere parameters
#[derive(Debug, Clone)]
pub struct AtmosphereParams {
    // Geometry
    pub ground_radius: f32,
    pub atmosphere_height: f32,

    // Rayleigh scattering (air molecules)
    pub rayleigh_scattering: [f32; 3],
    pub rayleigh_scale_height: f32,

    // Mie scattering (aerosols)
    pub mie_scattering: f32,
    pub mie_extinction: f32,
    pub mie_g: f32,
    pub mie_scale_height: f32,

    // Ozone absorption
    pub ozone_absorption: [f32; 3],
    pub ozone_center_height: f32,
    pub ozone_width: f32,

    // Sun
    pub sun_intensity: f32,
    pub sun_angular_radius: f32,
}

impl Default for AtmosphereParams {
    fn default() -> Self {
        Self::earth()
    }
}

impl AtmosphereParams {
    /// Earth-like atmosphere
    pub fn earth() -> Self {
        Self {
            ground_radius: 6360.0,
            atmosphere_height: 6420.0,
            rayleigh_scattering: [5.802e-6, 13.558e-6, 33.1e-6],
            rayleigh_scale_height: 8.0,
            mie_scattering: 3.996e-6,
            mie_extinction: 4.44e-6,
            mie_g: 0.8,
            mie_scale_height: 1.2,
            ozone_absorption: [0.65e-6, 1.881e-6, 0.085e-6],
            ozone_center_height: 25.0,
            ozone_width: 15.0,
            sun_intensity: 1.0,
            sun_angular_radius: 0.004675,
        }
    }

    /// Mars-like atmosphere
    pub fn mars() -> Self {
        Self {
            ground_radius: 3389.5,
            atmosphere_height: 3500.0,
            rayleigh_scattering: [19.918e-6, 13.57e-6, 5.75e-6],
            rayleigh_scale_height: 11.1,
            mie_scattering: 21e-6,
            mie_extinction: 25e-6,
            mie_g: 0.76,
            mie_scale_height: 11.1,
            ozone_absorption: [0.0; 3],
            ozone_center_height: 0.0,
            ozone_width: 0.0,
            sun_intensity: 0.43,
            sun_angular_radius: 0.00175,
        }
    }
}

/// Sun position calculator
pub struct SunPosition;

impl SunPosition {
    /// Calculate sun direction from time and location
    pub fn calculate(
        day_of_year: f32,
        time_of_day: f32, // 0-24
        latitude: f32,
        longitude: f32,
    ) -> [f32; 3] {
        let lat = latitude.to_radians();
        let lng = longitude.to_radians();

        // Solar declination
        let declination =
            23.45f32.to_radians() * ((360.0 * (284.0 + day_of_year) / 365.0).to_radians()).sin();

        // Hour angle
        let hour_angle = ((time_of_day - 12.0) * 15.0).to_radians() + lng;

        // Sun elevation
        let sin_elevation =
            lat.sin() * declination.sin() + lat.cos() * declination.cos() * hour_angle.cos();
        let elevation = sin_elevation.asin();

        // Sun azimuth
        let cos_azimuth =
            (declination.sin() - lat.sin() * sin_elevation) / (lat.cos() * elevation.cos());
        let mut azimuth = cos_azimuth.clamp(-1.0, 1.0).acos();

        if hour_angle.sin() > 0.0 {
            azimuth = core::f32::consts::TAU - azimuth;
        }

        // Convert to direction vector
        [
            -azimuth.sin() * elevation.cos(),
            elevation.sin(),
            -azimuth.cos() * elevation.cos(),
        ]
    }
}

/// Volumetric clouds
#[derive(Debug, Clone)]
pub struct VolumetricClouds {
    pub enabled: bool,
    pub coverage: f32,
    pub density: f32,
    pub altitude: f32,
    pub thickness: f32,
    pub wind_direction: [f32; 2],
    pub wind_speed: f32,
    pub detail_scale: f32,
    pub detail_intensity: f32,
}

impl Default for VolumetricClouds {
    fn default() -> Self {
        Self {
            enabled: true,
            coverage: 0.5,
            density: 1.0,
            altitude: 2000.0,
            thickness: 500.0,
            wind_direction: [1.0, 0.0],
            wind_speed: 5.0,
            detail_scale: 0.5,
            detail_intensity: 0.3,
        }
    }
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
