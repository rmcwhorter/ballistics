//! Gyroscopic stability (Miller), spin drift and crosswind aerodynamic jump (Litz).
//!
//! These are the empirical corrections used by Applied Ballistics on top of a point-mass
//! trajectory. They need the bullet's length and the barrel twist, which a point-mass
//! model otherwise never sees.

/// Miller twist-rule gyroscopic stability factor, corrected for velocity and air density.
pub fn miller_sg(
    mass_gr: f64,
    diameter_in: f64,
    length_in: f64,
    twist_in: f64,
    velocity_fps: f64,
    temp_f: f64,
    pressure_inhg: f64,
) -> f64 {
    let t = twist_in.abs() / diameter_in;
    let l = length_in / diameter_in;
    let sg = 30.0 * mass_gr / (t * t * diameter_in.powi(3) * l * (1.0 + l * l));
    let fv = (velocity_fps / 2800.0).cbrt();
    let fa = ((temp_f + 460.0) / 519.0) * (29.92 / pressure_inhg);
    sg * fv * fa
}

/// Litz's empirical spin drift, inches, at time of flight `tof_s`. Positive is the
/// direction of twist (right for a right-hand barrel).
pub fn spin_drift_in(sg: f64, tof_s: f64) -> f64 {
    1.25 * (sg + 1.2) * tof_s.powf(1.83)
}

/// Litz's crosswind aerodynamic jump, MOA of vertical deflection per mph of crosswind.
/// For a right-hand twist a wind from the left jumps the bullet up.
pub fn aero_jump_moa_per_mph(sg: f64, length_in: f64, diameter_in: f64) -> f64 {
    0.01 * sg - 0.0024 * (length_in / diameter_in) + 0.032
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m855_in_1_in_7_is_stable() {
        let sg = miller_sg(62.0, 0.224, 0.906, 7.0, 2800.0, 59.0, 29.92);
        assert!(sg > 1.5 && sg < 3.0, "sg = {sg}");
    }

    #[test]
    fn long_77_in_1_in_9_is_marginal() {
        let sg = miller_sg(77.0, 0.224, 0.99, 9.0, 2700.0, 59.0, 29.92);
        assert!(sg < 1.4, "sg = {sg}");
    }
}
