//! Moist-air atmosphere: density and speed of sound from station conditions.

pub const R_DRY: f64 = 287.058; // J/(kg K)
pub const R_VAPOR: f64 = 461.495; // J/(kg K)
pub const GAMMA: f64 = 1.4;
pub const P_SEA: f64 = 101_325.0; // Pa
pub const T_SEA: f64 = 288.15; // K
pub const LAPSE: f64 = 0.0065; // K/m

#[derive(Clone, Copy, Debug)]
pub struct Atmosphere {
    /// Air temperature at the shooter, K.
    pub temp_k: f64,
    /// Station (absolute, not sea-level-corrected) pressure, Pa.
    pub pressure_pa: f64,
    /// Relative humidity, 0..1.
    pub humidity: f64,
}

impl Atmosphere {
    /// ICAO standard sea level: 59 F, 29.92 inHg, dry.
    pub fn icao() -> Self {
        Self {
            temp_k: T_SEA,
            pressure_pa: P_SEA,
            humidity: 0.0,
        }
    }

    /// ICAO standard atmosphere at the given altitude (temperature and pressure both
    /// follow the standard lapse).
    pub fn icao_at_altitude_ft(alt_ft: f64) -> Self {
        let h = alt_ft * 0.3048;
        let t = T_SEA - LAPSE * h;
        let p = P_SEA * (t / T_SEA).powf(9.80665 / (R_DRY * LAPSE));
        Self {
            temp_k: t,
            pressure_pa: p,
            humidity: 0.0,
        }
    }

    /// Station conditions in shooter units.
    pub fn from_imperial(temp_f: f64, station_inhg: f64, humidity_pct: f64) -> Self {
        Self {
            temp_k: (temp_f - 32.0) * 5.0 / 9.0 + 273.15,
            pressure_pa: station_inhg * 3386.389,
            humidity: humidity_pct / 100.0,
        }
    }

    pub fn with_temp_f(mut self, temp_f: f64) -> Self {
        self.temp_k = (temp_f - 32.0) * 5.0 / 9.0 + 273.15;
        self
    }

    pub fn temp_f(&self) -> f64 {
        (self.temp_k - 273.15) * 9.0 / 5.0 + 32.0
    }

    pub fn pressure_inhg(&self) -> f64 {
        self.pressure_pa / 3386.389
    }

    /// Partial pressure of water vapor, Pa (Arden Buck saturation curve).
    pub fn vapor_pressure(&self) -> f64 {
        let tc = self.temp_k - 273.15;
        let es_hpa = 6.1121 * ((18.678 - tc / 234.5) * (tc / (257.14 + tc))).exp();
        self.humidity * es_hpa * 100.0
    }

    /// Air density, kg/m^3 (dry-air and vapor partial densities).
    pub fn density(&self) -> f64 {
        let e = self.vapor_pressure();
        (self.pressure_pa - e) / (R_DRY * self.temp_k) + e / (R_VAPOR * self.temp_k)
    }

    /// Speed of sound, m/s, using the virtual temperature for humid air.
    pub fn speed_of_sound(&self) -> f64 {
        let e = self.vapor_pressure();
        let tv = self.temp_k / (1.0 - 0.378 * e / self.pressure_pa);
        (GAMMA * R_DRY * tv).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icao_sea_level() {
        let a = Atmosphere::icao();
        assert!((a.density() - 1.2250).abs() < 2e-4);
        assert!((a.speed_of_sound() - 340.29).abs() < 0.05);
    }

    #[test]
    fn humid_air_is_lighter() {
        let dry = Atmosphere::from_imperial(80.0, 29.92, 0.0);
        let wet = Atmosphere::from_imperial(80.0, 29.92, 100.0);
        assert!(wet.density() < dry.density());
        assert!(wet.speed_of_sound() > dry.speed_of_sound());
    }

    #[test]
    fn standard_altitude() {
        // ICAO table: 5000 ft -> 24.90 inHg, 41.2 F
        let a = Atmosphere::icao_at_altitude_ft(5000.0);
        assert!((a.pressure_inhg() - 24.90).abs() < 0.02);
        assert!((a.temp_f() - 41.17).abs() < 0.05);
    }
}
