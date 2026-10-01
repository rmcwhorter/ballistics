//! Cross-checks against an independent solver, and the reticle's stated design point.
//!
//! Reference numbers come from py-ballisticcalc 2.3.1 (RK4 engine, ICAO atmosphere), produced
//! by `scripts/xcheck_pbc.py`: G7 BC, 2.75" sight height, 100 yd zero, 10 mph full-value wind
//! from 9 o'clock, no spin drift (twist 0). py-ballisticcalc reports wind drift as negative
//! for that wind; it is compared here by magnitude.

use ballistics::atmo::Atmosphere;
use ballistics::drag::DragTable;
use ballistics::reticle::{BDC, WIND, WIND15};
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
        assert!(
            (p.elevation_in - e).abs() < 0.1 + 2e-4 * e.abs(),
            "elevation {ctx}"
        );
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

fn bdc_miss_mil(mv: f64, zero_yd: f64) -> Vec<(f64, f64)> {
    let s = Solver::default();
    let p = bullet(77.0, 0.990, 0.190);
    let zero = Zero {
        range_yd: zero_yd,
        offset_in: 0.0,
        atmo: Atmosphere::icao(),
    };
    let angle = s.bore_angle(&p, &RIFLE, mv, &zero);
    let mut cond = Conditions::standard();
    cond.spin_drift = false;
    let marks: Vec<_> = BDC.iter().filter(|m| m.range_yd >= 300.0).collect();
    let ranges: Vec<f64> = marks.iter().map(|m| m.range_yd).collect();
    marks
        .iter()
        .zip(s.trajectory(&p, &RIFLE, mv, angle, &cond, &ranges))
        .map(|(m, pt)| (m.range_yd, pt.elevation_mil + m.mil))
        .collect()
}

/// The manual says the BDC is built around Mk262 (77gr, 2750 fps) with a 50 yd zero. That
/// load sits on the 400-600 marks, but the mark spacing is wrong for it: the 300 mark is
/// ~0.2 mil low of the bullet and the 800 mark ~0.2 mil high, a 0.45 mil tilt.
#[test]
fn bdc_matches_mk262_with_50_yd_zero_through_600() {
    let m = bdc_miss_mil(2750.0, 50.0);
    for &(r, miss) in &m {
        let tol = if (400.0..=600.0).contains(&r) {
            0.10
        } else {
            0.25
        };
        assert!(miss.abs() < tol, "{r} yd miss {miss:.3} mil");
    }
    let tilt = m[0].1 - m[m.len() - 1].1;
    assert!(tilt > 0.4, "300 vs 800 tilt {tilt:.3}");
}

/// The spacing of the marks fits ~2820-2860 fps, but at those speeds a 50 yd zero puts every
/// mark ~0.3 mil off. So no 50 yd zero fits all marks within 0.13 mil, at any velocity...
#[test]
fn no_50_yd_zero_fits_every_mark_tightly() {
    for mv in (2600..=3000).step_by(10) {
        let worst = bdc_miss_mil(mv as f64, 50.0)
            .iter()
            .fold(0.0_f64, |a, m| a.max(m.1.abs()));
        assert!(worst > 0.13, "{mv} fps fits within {worst:.3} mil");
    }
}

/// ...while a 100 yd zero at ~2860 fps fits every mark from 300 to 800 flat. So the marks
/// alone don't confirm the manual's stated design point.
#[test]
fn bdc_fits_77gr_with_100_yd_zero_flat() {
    for (r, miss) in bdc_miss_mil(2860.0, 100.0) {
        assert!(miss.abs() < 0.13, "{r} yd miss {miss:.3} mil");
    }
}

/// The wind dots are drawn for 5 and 10 mph and the row numerals for 15 mph; for a 77gr
/// load near design velocity they should be within 5% of that.
#[test]
fn wind_holds_fit_77gr() {
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
    for (w, pt) in WIND
        .iter()
        .zip(s.trajectory(&p, &RIFLE, 2650.0, angle, &cond, &ranges))
    {
        let per_mph = pt.windage_mil / 10.0;
        assert!((w.1 / per_mph - 5.0).abs() < 0.25, "{} yd 5 mph dot", w.0);
        assert!((w.2 / per_mph - 10.0).abs() < 0.5, "{} yd 10 mph dot", w.0);
        if let Some(n) = WIND15.iter().find(|n| n.0 == w.0) {
            assert!(
                (n.1 / per_mph - 15.0).abs() < 0.75,
                "{} yd 15 mph numeral",
                w.0
            );
        }
    }
}
