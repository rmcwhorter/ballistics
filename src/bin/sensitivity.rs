//! Impact sensitivities on a grid of muzzle velocities, for Monte Carlo sampling.
//!
//! The rifle is zeroed at 50 yd at the load's nominal velocity, so a fast or slow round
//! lands high or low. Wind effects are reported per mph of full-value crosswind (they are
//! linear in wind speed to well under 1%), so a sampler can draw velocity and wind
//! independently without re-integrating.
//!
//!   cargo run --release --bin sensitivity > out/sensitivity.csv

use ballistics::atmo::Atmosphere;
use ballistics::drag::DragTable;
use ballistics::reticle::BDC;
use ballistics::solver::*;

fn main() {
    let rifle = Rifle {
        sight_height_in: 2.75,
        twist_in: 7.0,
    };
    let p = |name: &str, gr, len, bc| Projectile {
        name: name.into(),
        mass_gr: gr,
        diameter_in: 0.224,
        length_in: len,
        bc,
        drag: DragTable::g7(),
    };
    // (projectile, nominal MV, MV standard deviation for the grid span)
    let loads = [
        (p("M193 55gr", 55.0, 0.750, 0.120), 3032.0, 35.0),
        (p("Hornady 77gr OTM", 77.0, 0.990, 0.190), 2600.0, 15.0),
    ];
    let ranges: Vec<f64> = BDC
        .iter()
        .filter(|m| m.range_yd % 100.0 == 0.0)
        .map(|m| m.range_yd)
        .collect();
    let s = Solver::default();
    let atmo = Atmosphere::icao();

    println!(
        "load,mv_nominal,mv,range_yd,hold_mil,elev_in,drift_in_per_mph,jump_in_per_mph,spin_in,tof_s"
    );
    for (proj, mv0, sd) in &loads {
        let zero = Zero {
            range_yd: 50.0,
            offset_in: 0.0,
            atmo,
        };
        let angle = s.bore_angle(proj, &rifle, *mv0, &zero);
        for i in -30..=30 {
            let mv = mv0 + sd * i as f64 / 6.0; // +/- 5 SD
            let mut still = Conditions::standard();
            still.atmo = atmo;
            let calm = s.trajectory(proj, &rifle, mv, angle, &still, &ranges);
            let mut windy = still;
            windy.wind = Wind::crosswind_from_left(10.0);
            let w = s.trajectory(proj, &rifle, mv, angle, &windy, &ranges);
            for ((r, c), w) in ranges.iter().zip(&calm).zip(&w) {
                let hold = BDC.iter().find(|m| m.range_yd == *r).unwrap().mil;
                println!(
                    "{},{},{:.2},{},{},{:.5},{:.6},{:.6},{:.5},{:.5}",
                    proj.name,
                    mv0,
                    mv,
                    r,
                    hold,
                    c.elevation_in,
                    (w.windage_in - c.windage_in) / 10.0,
                    (w.aero_jump_in - c.aero_jump_in) / 10.0,
                    c.spin_drift_in,
                    c.time_s
                );
            }
        }
    }
}
