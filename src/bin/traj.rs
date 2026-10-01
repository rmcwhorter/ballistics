//! General trajectory table.
//!
//!   cargo run --release --bin traj -- --bc 0.151 --drag g7 --mass 62 --len 0.906 --mv 2808 \
//!       --zero 100 --wind 10 --wind-clock 9 --max 800 --step 50

use ballistics::atmo::Atmosphere;
use ballistics::drag::DragTable;
use ballistics::solver::*;

fn main() {
    let mut a = Args(std::env::args().skip(1).collect());
    if a.flag("--help") || a.flag("-h") {
        println!(
            "traj [--bc 0.151] [--drag g7|g1] [--mass 62] [--diam 0.224] [--len 0.906] [--mv 2808]\n\
             \x20    [--sh 2.75] [--twist 7] [--zero 100] [--zero-offset 0]\n\
             \x20    [--alt FT | --temp F --press INHG_STATION] [--rh %]\n\
             \x20    [--wind MPH] [--wind-clock 9] [--look DEG] [--lat DEG --az DEG]\n\
             \x20    [--max 800] [--step 50] [--no-spin] [--no-jump] [--csv]"
        );
        return;
    }
    let drag = match a.str("--drag", "g7").to_lowercase().as_str() {
        "g1" => DragTable::g1(),
        "g7" => DragTable::g7(),
        other => panic!("unknown drag model {other}"),
    };
    let proj = Projectile {
        name: "custom".into(),
        mass_gr: a.num("--mass", 62.0),
        diameter_in: a.num("--diam", 0.224),
        length_in: a.num("--len", 0.906),
        bc: a.num("--bc", 0.151),
        drag,
    };
    let rifle = Rifle {
        sight_height_in: a.num("--sh", 2.75),
        twist_in: a.num("--twist", 7.0),
    };
    let mv = a.num("--mv", 2808.0);

    let mut atmo = match a.opt("--alt") {
        Some(alt) => Atmosphere::icao_at_altitude_ft(alt),
        None => Atmosphere::icao(),
    };
    if let Some(t) = a.opt("--temp") {
        atmo = atmo.with_temp_f(t);
    }
    if let Some(p) = a.opt("--press") {
        atmo.pressure_pa = p * 3386.389;
    }
    atmo.humidity = a.num("--rh", 0.0) / 100.0;

    let zero = Zero {
        range_yd: a.num("--zero", 100.0),
        offset_in: a.num("--zero-offset", 0.0),
        atmo,
    };
    let cond = Conditions {
        atmo,
        wind: Wind {
            speed_mph: a.num("--wind", 0.0),
            from_clock: a.num("--wind-clock", 9.0),
        },
        look_angle_deg: a.num("--look", 0.0),
        coriolis: a.opt("--lat").map(|lat| Coriolis {
            latitude_deg: lat,
            azimuth_deg: a.num("--az", 0.0),
        }),
        spin_drift: !a.flag("--no-spin"),
        aero_jump: !a.flag("--no-jump"),
    };
    let (max, step) = (a.num("--max", 800.0), a.num("--step", 50.0));
    let csv = a.flag("--csv");
    if let Some(left) = a.0.first() {
        panic!("unrecognized argument {left}");
    }

    let solver = Solver::default();
    let angle = solver.bore_angle(&proj, &rifle, mv, &zero);
    let ranges: Vec<f64> = (0..)
        .map(|i| i as f64 * step)
        .take_while(|&r| r <= max + 1e-9)
        .collect();
    let pts = solver.trajectory(&proj, &rifle, mv, angle, &cond, &ranges);

    if csv {
        println!(
            "range_yd,time_s,velocity_fps,mach,energy_ftlb,elev_in,elev_mil,wind_in,wind_mil,spin_drift_in,aero_jump_in"
        );
        for p in &pts {
            println!(
                "{},{:.5},{:.2},{:.4},{:.1},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4}",
                p.range_yd,
                p.time_s,
                p.velocity_fps,
                p.mach,
                p.energy_ftlb,
                p.elevation_in,
                p.elevation_mil,
                p.windage_in,
                p.windage_mil,
                p.spin_drift_in,
                p.aero_jump_in
            );
        }
        return;
    }
    println!(
        "bore angle {:.3} MOA | air {:.4} kg/m3, c {:.1} m/s | {:.1} F, {:.2} inHg station",
        angle / MOA_RAD,
        atmo.density(),
        atmo.speed_of_sound(),
        atmo.temp_f(),
        atmo.pressure_inhg()
    );
    println!(" range   TOF    vel  mach  energy   elev(in) elev(mil)   wind(in) wind(mil)");
    for p in &pts {
        println!(
            "{:6.0} {:5.3} {:6.0} {:5.2} {:7.0} {:10.2} {:9.2} {:10.2} {:9.2}",
            p.range_yd,
            p.time_s,
            p.velocity_fps,
            p.mach,
            p.energy_ftlb,
            p.elevation_in,
            p.elevation_mil,
            p.windage_in,
            p.windage_mil
        );
    }
}

struct Args(Vec<String>);

impl Args {
    fn take(&mut self, name: &str) -> Option<String> {
        let i = self.0.iter().position(|a| a == name)?;
        self.0.remove(i);
        assert!(i < self.0.len(), "{name} needs a value");
        Some(self.0.remove(i))
    }
    fn opt(&mut self, name: &str) -> Option<f64> {
        self.take(name).map(|v| {
            v.parse()
                .unwrap_or_else(|_| panic!("bad number for {name}: {v}"))
        })
    }
    fn num(&mut self, name: &str, default: f64) -> f64 {
        self.opt(name).unwrap_or(default)
    }
    fn str(&mut self, name: &str, default: &str) -> String {
        self.take(name).unwrap_or_else(|| default.to_string())
    }
    fn flag(&mut self, name: &str) -> bool {
        match self.0.iter().position(|a| a == name) {
            Some(i) => {
                self.0.remove(i);
                true
            }
            None => false,
        }
    }
}
