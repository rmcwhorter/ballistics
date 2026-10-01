//! 3-DOF point-mass trajectory with a standard-drag-function BC, RK4 integration,
//! moist atmosphere, 3-D wind, Coriolis/Eotvos, look angle, and Litz spin drift and
//! aerodynamic jump layered on top.
//!
//! Frame: x along the line of sight (downrange), y perpendicular to it (up), z to the
//! shooter's right. The sight sits at the origin; the bore starts `sight_height` below.

use crate::atmo::Atmosphere;
use crate::drag::DragTable;
use crate::stability;

pub const G: f64 = 9.80665;
pub const OMEGA_EARTH: f64 = 7.292_115e-5;
pub const YD: f64 = 0.9144;
pub const INCH: f64 = 0.0254;
pub const FPS: f64 = 0.3048;
pub const MPH: f64 = 0.44704;
pub const GRAIN: f64 = 6.479_891e-5;
/// lb/in^2 -> kg/m^2, for converting a BC to SI.
pub const BC_SI: f64 = 703.069_6;
pub const MOA_RAD: f64 = std::f64::consts::PI / (180.0 * 60.0);

#[derive(Clone, Debug)]
pub struct Projectile {
    pub name: String,
    pub mass_gr: f64,
    pub diameter_in: f64,
    pub length_in: f64,
    /// Ballistic coefficient against `drag`, lb/in^2.
    pub bc: f64,
    pub drag: DragTable,
}

#[derive(Clone, Copy, Debug)]
pub struct Rifle {
    /// Center of the optic above the bore axis, inches.
    pub sight_height_in: f64,
    /// Inches per turn; positive is right-hand twist.
    pub twist_in: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Wind {
    pub speed_mph: f64,
    /// Clock direction the wind comes FROM: 12 is a headwind, 3 blows right-to-left,
    /// 9 blows left-to-right.
    pub from_clock: f64,
}

impl Wind {
    pub fn crosswind_from_left(mph: f64) -> Self {
        Self {
            speed_mph: mph,
            from_clock: 9.0,
        }
    }

    /// Air velocity (x, z) in m/s.
    fn velocity(&self) -> (f64, f64) {
        let th = self.from_clock * std::f64::consts::PI / 6.0;
        (
            -self.speed_mph * MPH * th.cos(),
            -self.speed_mph * MPH * th.sin(),
        )
    }

    /// Crosswind component blowing toward the shooter's right, mph.
    pub fn crosswind_mph(&self) -> f64 {
        self.velocity().1 / MPH
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Coriolis {
    pub latitude_deg: f64,
    /// Direction of fire, degrees clockwise from true north.
    pub azimuth_deg: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Conditions {
    pub atmo: Atmosphere,
    pub wind: Wind,
    /// Positive is uphill.
    pub look_angle_deg: f64,
    pub coriolis: Option<Coriolis>,
    pub spin_drift: bool,
    pub aero_jump: bool,
}

impl Conditions {
    pub fn standard() -> Self {
        Self {
            atmo: Atmosphere::icao(),
            wind: Wind::default(),
            look_angle_deg: 0.0,
            coriolis: None,
            spin_drift: true,
            aero_jump: true,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Zero {
    pub range_yd: f64,
    /// Point of impact above point of aim at `range_yd`, inches (e.g. "1 inch high at 100").
    pub offset_in: f64,
    pub atmo: Atmosphere,
}

#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub range_yd: f64,
    pub time_s: f64,
    pub velocity_fps: f64,
    pub mach: f64,
    pub energy_ftlb: f64,
    /// Impact relative to the line of sight, inches; positive is high.
    pub elevation_in: f64,
    /// Positive is right.
    pub windage_in: f64,
    /// Angular impact relative to the line of sight, milliradians.
    pub elevation_mil: f64,
    pub windage_mil: f64,
    /// Portions of the above that came from the empirical corrections, inches.
    pub spin_drift_in: f64,
    pub aero_jump_in: f64,
}

#[derive(Clone, Copy, Debug)]
struct State {
    t: f64,
    p: [f64; 3],
    v: [f64; 3],
}

pub struct Solver {
    /// Integration step, seconds.
    pub dt: f64,
    pub max_time_s: f64,
}

impl Default for Solver {
    fn default() -> Self {
        Self {
            dt: 5e-5,
            max_time_s: 6.0,
        }
    }
}

impl Solver {
    /// Bore elevation above the line of sight (radians) that puts the zero where asked.
    /// Zeroing is done level, in still air, under the zero atmosphere.
    pub fn bore_angle(&self, proj: &Projectile, rifle: &Rifle, mv_fps: f64, zero: &Zero) -> f64 {
        let cond = Conditions {
            atmo: zero.atmo,
            wind: Wind::default(),
            look_angle_deg: 0.0,
            coriolis: None,
            spin_drift: false,
            aero_jump: false,
        };
        let x = zero.range_yd * YD;
        let target = zero.offset_in * INCH;
        let mut theta = (rifle.sight_height_in * INCH) / x;
        for _ in 0..30 {
            let states = self.fly(proj, rifle, mv_fps, theta, &cond, x);
            let y = interp(&states, x).p[1];
            let err = y - target;
            if err.abs() < 1e-9 {
                break;
            }
            // dy/dtheta is very nearly x for a flat trajectory.
            theta -= err / x;
        }
        theta
    }

    pub fn trajectory(
        &self,
        proj: &Projectile,
        rifle: &Rifle,
        mv_fps: f64,
        bore_angle: f64,
        cond: &Conditions,
        ranges_yd: &[f64],
    ) -> Vec<Point> {
        let max_x = ranges_yd.iter().cloned().fold(0.0, f64::max) * YD;
        let states = self.fly(proj, rifle, mv_fps, bore_angle, cond, max_x + 1.0);
        let sg = stability::miller_sg(
            proj.mass_gr,
            proj.diameter_in,
            proj.length_in,
            rifle.twist_in,
            mv_fps,
            cond.atmo.temp_f(),
            cond.atmo.pressure_inhg(),
        );
        let twist_sign = rifle.twist_in.signum();
        let jump_rad = if cond.aero_jump {
            stability::aero_jump_moa_per_mph(sg, proj.length_in, proj.diameter_in)
                * cond.wind.crosswind_mph()
                * twist_sign
                * MOA_RAD
        } else {
            0.0
        };
        let c = cond.atmo.speed_of_sound();
        let mass_kg = proj.mass_gr * GRAIN;

        ranges_yd
            .iter()
            .map(|&r| {
                let x = r * YD;
                let s = interp(&states, x);
                let speed = norm(s.v);
                let sd = if cond.spin_drift {
                    twist_sign * stability::spin_drift_in(sg, s.t) * INCH
                } else {
                    0.0
                };
                let aj = jump_rad * x;
                let y = s.p[1] + aj;
                let z = s.p[2] + sd;
                Point {
                    range_yd: r,
                    time_s: s.t,
                    velocity_fps: speed / FPS,
                    mach: speed / c,
                    energy_ftlb: 0.5 * mass_kg * speed * speed / 1.355_818,
                    elevation_in: y / INCH,
                    windage_in: z / INCH,
                    elevation_mil: if x > 0.0 { y.atan2(x) * 1000.0 } else { 0.0 },
                    windage_mil: if x > 0.0 { z.atan2(x) * 1000.0 } else { 0.0 },
                    spin_drift_in: sd / INCH,
                    aero_jump_in: aj / INCH,
                }
            })
            .collect()
    }

    /// Integrate until the bullet passes `max_x` meters downrange.
    fn fly(
        &self,
        proj: &Projectile,
        rifle: &Rifle,
        mv_fps: f64,
        bore_angle: f64,
        cond: &Conditions,
        max_x: f64,
    ) -> Vec<State> {
        let look = cond.look_angle_deg.to_radians();
        // Gravity expressed in the line-of-sight frame.
        let g = [-G * look.sin(), -G * look.cos(), 0.0];
        let (wx, wz) = cond.wind.velocity();
        // Wind is horizontal; rotate its downrange part into the LOS frame.
        let wind = [wx * look.cos(), -wx * look.sin(), wz];
        let omega = cond.coriolis.map(|c| {
            let (lat, az) = (c.latitude_deg.to_radians(), c.azimuth_deg.to_radians());
            // Earth's rotation in the level (downrange, up, right) frame...
            let (ox, oy, oz) = (
                OMEGA_EARTH * lat.cos() * az.cos(),
                OMEGA_EARTH * lat.sin(),
                -OMEGA_EARTH * lat.cos() * az.sin(),
            );
            // ...then tilted by the look angle.
            [
                ox * look.cos() + oy * look.sin(),
                -ox * look.sin() + oy * look.cos(),
                oz,
            ]
        });
        let rho = cond.atmo.density();
        let c = cond.atmo.speed_of_sound();
        let k = rho * std::f64::consts::PI / (8.0 * proj.bc * BC_SI);

        let accel = |v: [f64; 3]| -> [f64; 3] {
            let vr = [v[0] - wind[0], v[1] - wind[1], v[2] - wind[2]];
            let speed = norm(vr);
            let drag = k * proj.drag.cd(speed / c) * speed;
            let mut a = [
                g[0] - drag * vr[0],
                g[1] - drag * vr[1],
                g[2] - drag * vr[2],
            ];
            if let Some(o) = omega {
                // -2 (Omega x v)
                a[0] -= 2.0 * (o[1] * v[2] - o[2] * v[1]);
                a[1] -= 2.0 * (o[2] * v[0] - o[0] * v[2]);
                a[2] -= 2.0 * (o[0] * v[1] - o[1] * v[0]);
            }
            a
        };

        let mv = mv_fps * FPS;
        let mut s = State {
            t: 0.0,
            p: [0.0, -rifle.sight_height_in * INCH, 0.0],
            v: [mv * bore_angle.cos(), mv * bore_angle.sin(), 0.0],
        };
        let mut out = Vec::with_capacity((max_x / (mv * self.dt)) as usize + 16);
        out.push(s);
        let h = self.dt;
        while s.p[0] < max_x && s.t < self.max_time_s {
            // Position derivative is velocity; velocity derivative depends only on velocity.
            let k1v = accel(s.v);
            let v2 = add(s.v, k1v, 0.5 * h);
            let k2v = accel(v2);
            let v3 = add(s.v, k2v, 0.5 * h);
            let k3v = accel(v3);
            let v4 = add(s.v, k3v, h);
            let k4v = accel(v4);
            for i in 0..3 {
                s.p[i] += h / 6.0 * (s.v[i] + 2.0 * v2[i] + 2.0 * v3[i] + v4[i]);
                s.v[i] += h / 6.0 * (k1v[i] + 2.0 * k2v[i] + 2.0 * k3v[i] + k4v[i]);
            }
            s.t += h;
            out.push(s);
        }
        out
    }
}

fn norm(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn add(a: [f64; 3], b: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] + b[0] * s, a[1] + b[1] * s, a[2] + b[2] * s]
}

/// Cubic Hermite interpolation of the state at downrange distance `x`.
fn interp(states: &[State], x: f64) -> State {
    let i = states.partition_point(|s| s.p[0] < x);
    if i == 0 {
        return states[0];
    }
    assert!(
        i < states.len(),
        "trajectory ended before {x} m (out of time?)"
    );
    let (a, b) = (states[i - 1], states[i]);
    let h = b.t - a.t;
    // Solve x(t) = x on the Hermite cubic for x (x is monotone, so Newton converges).
    let mut u = (x - a.p[0]) / (b.p[0] - a.p[0]);
    let herm = |u: f64, p0: f64, p1: f64, m0: f64, m1: f64| {
        let (u2, u3) = (u * u, u * u * u);
        (2.0 * u3 - 3.0 * u2 + 1.0) * p0
            + (u3 - 2.0 * u2 + u) * h * m0
            + (-2.0 * u3 + 3.0 * u2) * p1
            + (u3 - u2) * h * m1
    };
    for _ in 0..5 {
        let f = herm(u, a.p[0], b.p[0], a.v[0], b.v[0]) - x;
        let dfdu = h * (a.v[0] + (b.v[0] - a.v[0]) * u);
        u -= f / dfdu;
    }
    let mut s = State {
        t: a.t + u * h,
        p: [0.0; 3],
        v: [0.0; 3],
    };
    for k in 0..3 {
        s.p[k] = herm(u, a.p[k], b.p[k], a.v[k], b.v[k]);
        s.v[k] = a.v[k] + (b.v[k] - a.v[k]) * u;
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m855() -> Projectile {
        Projectile {
            name: "M855".into(),
            mass_gr: 62.0,
            diameter_in: 0.224,
            length_in: 0.906,
            bc: 0.151,
            drag: DragTable::g7(),
        }
    }

    const RIFLE: Rifle = Rifle {
        sight_height_in: 2.75,
        twist_in: 7.0,
    };

    fn zero100() -> Zero {
        Zero {
            range_yd: 100.0,
            offset_in: 0.0,
            atmo: Atmosphere::icao(),
        }
    }

    #[test]
    fn zero_lands_on_zero() {
        let s = Solver::default();
        let a = s.bore_angle(&m855(), &RIFLE, 2808.0, &zero100());
        let mut c = Conditions::standard();
        c.spin_drift = false;
        let p = s.trajectory(&m855(), &RIFLE, 2808.0, a, &c, &[0.0, 100.0]);
        assert!((p[0].elevation_in + 2.75).abs() < 1e-9);
        assert!(p[1].elevation_in.abs() < 1e-5);
    }

    #[test]
    fn step_size_converged() {
        let p = m855();
        let z = zero100();
        let run = |dt| {
            let s = Solver {
                dt,
                ..Solver::default()
            };
            let a = s.bore_angle(&p, &RIFLE, 2808.0, &z);
            s.trajectory(&p, &RIFLE, 2808.0, a, &Conditions::standard(), &[800.0])[0].elevation_in
        };
        assert!((run(5e-5) - run(2.5e-5)).abs() < 1e-4);
    }

    #[test]
    fn wind_from_left_pushes_right() {
        let s = Solver::default();
        let p = m855();
        let a = s.bore_angle(&p, &RIFLE, 2808.0, &zero100());
        let mut c = Conditions::standard();
        c.spin_drift = false;
        c.wind = Wind::crosswind_from_left(10.0);
        let pt = s.trajectory(&p, &RIFLE, 2808.0, a, &c, &[500.0])[0];
        assert!(pt.windage_in > 0.0);
        // Right-hand twist + wind from the left: aerodynamic jump is upward.
        assert!(pt.aero_jump_in > 0.0);
    }

    #[test]
    fn coriolis_deflects_right_in_northern_hemisphere() {
        let s = Solver::default();
        let p = m855();
        let a = s.bore_angle(&p, &RIFLE, 2808.0, &zero100());
        let mut c = Conditions::standard();
        c.spin_drift = false;
        c.coriolis = Some(Coriolis {
            latitude_deg: 45.0,
            azimuth_deg: 0.0,
        });
        let pt = s.trajectory(&p, &RIFLE, 2808.0, a, &c, &[800.0])[0];
        assert!(pt.windage_in > 0.0);
    }
}
