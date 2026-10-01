//! Cross-checks against an independent solver, and the reticle's stated design point.
//!
//! Reference numbers come from py-ballisticcalc 2.3.1 (RK4 engine, ICAO atmosphere), produced
//! by `scripts/xcheck_pbc.py`: G7 BC, 2.75" sight height, 100 yd zero, 10 mph full-value wind
//! from 9 o'clock, no spin drift (twist 0). py-ballisticcalc reports wind drift as negative
//! for that wind; it is compared here by magnitude.

use ballistics::atmo::Atmosphere;
use ballistics::drag::DragTable;
use ballistics::reticle::{BDC, WIND};
use ballistics::solver::*;

fn bullet(gr: f64, len: f64, bc: f64) -> Projectile {
    Projectile {
        name: format!("{gr}gr"),
        mass_gr: gr,
        diameter_in: 0.224,
        length_in: len,
        bc,
        drag: DragTable::g7(),
    }
}

const RIFLE: Rifle = Rifle {
    sight_height_in: 2.75,
    twist_in: 7.0,
};

/// (range yd, time s, velocity fps, elevation in, |windage| in)
type Row = (f64, f64, f64, f64, f64);

fn check(proj: Projectile, mv: f64, rows: &[Row]) {
    let s = Solver::default();
    let zero = Zero {
        range_yd: 100.0,
        offset_in: 0.0,
        atmo: Atmosphere::icao(),
    };
    let angle = s.bore_angle(&proj, &RIFLE, mv, &zero);
    let mut cond = Conditions::standard();
    cond.spin_drift = false;
    cond.aero_jump = false;
    cond.wind = Wind::crosswind_from_left(10.0);
    let ranges: Vec<f64> = rows.iter().map(|r| r.0).collect();
    let pts = s.trajectory(&proj, &RIFLE, mv, angle, &cond, &ranges);
    for (p, &(r, t, v, e, w)) in pts.iter().zip(rows) {
        let ctx = format!("{} @ {r} yd: ours {p:?}", proj.name);
        assert!((p.time_s - t).abs() < 2e-3, "TOF {ctx}");
        assert!((p.velocity_fps - v).abs() < 1.0, "velocity {ctx}");
        assert!((p.elevation_in - e).abs() < 0.1 + 2e-4 * e.abs(), "elevation {ctx}");
        assert!((p.windage_in - w).abs() < 0.05, "windage {ctx}");
    }
}

#[test]
fn m193_matches_py_ballisticcalc() {
    check(
        bullet(55.0, 0.750, 0.120),
        3032.0,
        &[
            (100.0, 0.1061, 2637.2, 0.000, 1.258),
            (300.0, 0.3718, 1936.3, -11.381, 13.188),
            (500.0, 0.7436, 1343.3, -61.614, 43.794),
            (800.0, 1.5943, 927.3, -307.765, 141.279),
        ],
    );
}

#[test]
fn m855_matches_py_ballisticcalc() {
    check(
        bullet(62.0, 0.906, 0.151),
        2808.0,
        &[
            (300.0, 0.3845, 1954.5, -12.743, 11.254),
            (500.0, 0.7379, 1473.2, -62.944, 35.855),
            (800.0, 1.5086, 1000.1, -278.979, 115.078),
        ],
    );
}

#[test]
fn hornady_77_matches_py_ballisticcalc() {
    check(
        bullet(77.0, 0.990, 0.190),
        2650.0,
        &[
            (300.0, 0.3930, 1981.0, -13.728, 9.398),
            (500.0, 0.7309, 1591.2, -63.516, 29.017),
            (800.0, 1.4167, 1090.9, -254.053, 89.939),
        ],
    );
}

/// The manual says the BDC is built around Mk262 (77gr, 2750 fps) with a 50 yd zero.
/// With the extracted subtensions, that load should sit on every mark from 300 to 800.
#[test]
fn bdc_is_drawn_for_mk262_with_50_yd_zero() {
    let s = Solver::default();
    let p = bullet(77.0, 0.990, 0.190);
    let zero = Zero {
        range_yd: 50.0,
        offset_in: 0.0,
        atmo: Atmosphere::icao(),
    };
    let angle = s.bore_angle(&p, &RIFLE, 2750.0, &zero);
    let mut cond = Conditions::standard();
    cond.spin_drift = false;
    let marks: Vec<_> = BDC.iter().filter(|m| m.range_yd >= 300.0).collect();
    let ranges: Vec<f64> = marks.iter().map(|m| m.range_yd).collect();
    for (m, pt) in marks.iter().zip(s.trajectory(&p, &RIFLE, 2750.0, angle, &cond, &ranges)) {
        let miss = pt.elevation_mil + m.mil;
        assert!(miss.abs() < 0.25, "{} yd miss {miss:.3} mil", m.range_yd);
    }
}

/// The wind dots are drawn for 5 and 10 mph; for a 77gr load near design velocity they
/// should be within half a mph of that.
#[test]
fn wind_dots_fit_77gr() {
    let s = Solver::default();
    let p = bullet(77.0, 0.990, 0.190);
    let zero = Zero {
        range_yd: 50.0,
        offset_in: 0.0,
        atmo: Atmosphere::icao(),
    };
    let angle = s.bore_angle(&p, &RIFLE, 2650.0, &zero);
    let mut cond = Conditions::standard();
    cond.spin_drift = false;
    cond.aero_jump = false;
    cond.wind = Wind::crosswind_from_left(10.0);
    let ranges: Vec<f64> = WIND.iter().map(|w| w.0).collect();
    for (w, pt) in WIND.iter().zip(s.trajectory(&p, &RIFLE, 2650.0, angle, &cond, &ranges)) {
        let per_mph = pt.windage_mil / 10.0;
        assert!((w.1 / per_mph - 5.0).abs() < 0.5, "{} yd 5 mph dot", w.0);
        assert!((w.2 / per_mph - 10.0).abs() < 0.5, "{} yd 10 mph dot", w.0);
    }
}
