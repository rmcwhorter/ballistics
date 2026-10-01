//! How well do M193, M855/SS109 and a 77gr OTM match the ACSS Raptor 5.56/.308 Yard G2
//! BDC (Primary Arms PLxC 1-8x24 FFP RDB)?
//!
//!   cargo run --release --bin bdc

use ballistics::atmo::Atmosphere;
use ballistics::drag::DragTable;
use ballistics::reticle::{BDC, LEADS, Mark, WIND, WIND15};
use ballistics::solver::*;

/// The user's rifle; the M193 and SS109 velocities are from it, and the 77gr is a guess for it.
const BARREL_IN: f64 = 14.5;
const SIGHT_HEIGHT_IN: f64 = 2.75;
const TWIST_IN: f64 = 7.0;
/// Half-height of the vital zone we call a hit, inches.
const TOL_IN: f64 = 6.0;

const BEST: &str = "best zero";

struct Load {
    proj: Projectile,
    mv: f64,
}

fn loads() -> Vec<Load> {
    let p = |name: &str, gr, len, bc| Projectile {
        name: name.into(),
        mass_gr: gr,
        diameter_in: 0.224,
        length_in: len,
        bc,
        drag: DragTable::g7(),
    };
    vec![
        Load {
            proj: p("M193 55gr FMJBT", 55.0, 0.750, 0.120),
            mv: 3032.0,
        },
        Load {
            proj: p("SS109/M855 62gr", 62.0, 0.906, 0.151),
            mv: 2808.0,
        },
        Load {
            proj: p("Hornady 77gr OTM", 77.0, 0.990, 0.190),
            // Mk262 is ~2750 from 20"; ~2600 is plausible from 14.5".
            mv: 2600.0,
        },
    ]
}

fn rifle() -> Rifle {
    Rifle {
        sight_height_in: SIGHT_HEIGHT_IN,
        twist_in: TWIST_IN,
    }
}

/// Elevation (mil, + high) at every yard out to 900 for one load/zero/atmosphere.
struct Curve {
    elev_mil: Vec<f64>,
    elev_in: Vec<f64>,
}

impl Curve {
    fn new(load: &Load, rifle: &Rifle, zero: &Zero, atmo: Atmosphere) -> Self {
        let s = Solver::default();
        let angle = s.bore_angle(&load.proj, rifle, load.mv, zero);
        let mut cond = Conditions::standard();
        cond.atmo = atmo;
        cond.spin_drift = false;
        let ranges: Vec<f64> = (0..=900).map(f64::from).collect();
        let pts = s.trajectory(&load.proj, rifle, load.mv, angle, &cond, &ranges);
        Self {
            elev_mil: pts.iter().map(|p| p.elevation_mil).collect(),
            elev_in: pts.iter().map(|p| p.elevation_in).collect(),
        }
    }

    /// Where the bullet goes (inches, + high) when this mark is held on a target at the
    /// mark's labeled range.
    fn miss_in(&self, m: &Mark) -> f64 {
        let r = m.range_yd as usize;
        self.elev_in[r] + (r as f64 * 36.0) * (m.mil / 1000.0).tan()
    }

    fn miss_mil(&self, m: &Mark) -> f64 {
        self.elev_mil[m.range_yd as usize] + m.mil
    }

    /// The range the mark is actually correct for (on the descending branch).
    fn true_range(&self, m: &Mark) -> Option<f64> {
        let apex = (1..self.elev_mil.len())
            .max_by(|&a, &b| self.elev_mil[a].total_cmp(&self.elev_mil[b]))
            .unwrap();
        let start = apex.max(60);
        (start..self.elev_mil.len() - 1).find_map(|r| {
            let (a, b) = (-self.elev_mil[r], -self.elev_mil[r + 1]);
            (a <= m.mil && b > m.mil).then(|| r as f64 + (m.mil - a) / (b - a))
        })
    }

    /// Near crossing of the line of sight (the "zero distance" a 100-yd offset implies).
    fn near_zero(&self) -> Option<f64> {
        (1..150).find_map(|r| {
            let (a, b) = (self.elev_in[r], self.elev_in[r + 1]);
            (a <= 0.0 && b > 0.0).then(|| r as f64 + -a / (b - a))
        })
    }
}

fn zero_at(range_yd: f64, offset_in: f64, atmo: Atmosphere) -> Zero {
    Zero {
        range_yd,
        offset_in,
        atmo,
    }
}

fn rms_mil(c: &Curve, marks: &[&Mark]) -> f64 {
    (marks.iter().map(|m| c.miss_mil(m).powi(2)).sum::<f64>() / marks.len() as f64).sqrt()
}

fn fit_marks() -> Vec<&'static Mark> {
    BDC.iter().filter(|m| m.range_yd >= 300.0).collect()
}

/// Every mark is within `tol` inches of the target out to this range.
fn usable_to(c: &Curve, tol: f64) -> (f64, f64) {
    let mut reach = 0.0;
    let mut worst: f64 = 0.0;
    // 200 is skipped: it has no physical mark, only a leader line in the manual.
    for m in BDC.iter().filter(|m| m.range_yd != 200.0) {
        let miss = c.miss_in(m).abs();
        if miss > tol {
            break;
        }
        reach = m.range_yd;
        worst = worst.max(miss);
    }
    (reach, worst)
}

/// 100-yard zero offset (inches high) that keeps every mark within `tol` the farthest,
/// breaking ties by the smallest worst-case miss inside that reach.
fn best_offset(load: &Load, rifle: &Rifle, atmo: Atmosphere, tol: f64) -> (f64, f64) {
    let mut best = (0.0, (0.0, f64::INFINITY));
    let mut off = -2.0;
    while off <= 6.0 + 1e-9 {
        let c = Curve::new(load, rifle, &zero_at(100.0, off, atmo), atmo);
        let (reach, worst) = usable_to(&c, tol);
        if reach > best.1.0 || (reach == best.1.0 && worst < best.1.1) {
            best = (off, (reach, worst));
        }
        off += 0.05;
    }
    (best.0, best.1.0)
}

/// Muzzle velocity at which the BDC is best matched with the given zero.
fn best_mv(load: &Load, rifle: &Rifle, zero: &Zero, atmo: Atmosphere) -> (f64, f64) {
    let marks = fit_marks();
    let mut best = (0.0, f64::INFINITY);
    let mut mv = 2300.0;
    while mv <= 3400.0 {
        let l = Load {
            proj: load.proj.clone(),
            mv,
        };
        let e = rms_mil(&Curve::new(&l, rifle, zero, atmo), &marks);
        if e < best.1 {
            best = (mv, e);
        }
        mv += 5.0;
    }
    best
}

fn main() {
    let rifle = rifle();
    let sea = Atmosphere::icao();
    if let Some(dir) = std::env::args()
        .nth(1)
        .filter(|a| a == "--csv")
        .and(std::env::args().nth(2))
    {
        write_csv(&dir, &rifle, sea);
        return;
    }
    println!("ACSS Raptor 5.56/.308 Yard G2 BDC vs. load");
    println!(
        "Rifle: {BARREL_IN}\" barrel, sight height {SIGHT_HEIGHT_IN}\", 1:{TWIST_IN} RH. Air: ICAO sea level (59 F, 29.92 inHg, dry) unless noted."
    );
    println!(
        "Miss = where the bullet lands when the mark is held dead on a target at the mark's range (+ high, - low)."
    );
    println!("True range = the range that mark is actually correct for.\n");

    for load in loads() {
        let sd = ballistics::stability::miller_sg(
            load.proj.mass_gr,
            load.proj.diameter_in,
            load.proj.length_in,
            TWIST_IN,
            load.mv,
            59.0,
            29.92,
        );
        println!(
            "=== {} @ {:.0} fps (G7 {:.3}, Sg {:.2} in 1:{TWIST_IN}) ===",
            load.proj.name, load.mv, load.proj.bc, sd
        );
        let (off, _) = best_offset(&load, &rifle, sea, TOL_IN);
        let zeros = [
            ("50 yd zero", zero_at(50.0, 0.0, sea)),
            ("100 yd zero", zero_at(100.0, 0.0, sea)),
            (BEST, zero_at(100.0, off, sea)),
        ];
        let curves: Vec<Curve> = zeros
            .iter()
            .map(|(_, z)| Curve::new(&load, &rifle, z, sea))
            .collect();
        let best_near = curves[2].near_zero();
        println!(
            "{BEST} = {:+.2}\" high at 100 yd (same as a {} yd zero)",
            off,
            best_near.map_or("--".into(), |r| format!("{r:.0}"))
        );
        print!("{:>5} {:>6} |", "range", "mark");
        for (n, _) in &zeros {
            print!(" {:^27} |", n);
        }
        println!();
        print!("{:>5} {:>6} |", "yd", "mil");
        for _ in &zeros {
            print!(" {:>8} {:>7} {:>9} |", "miss in", "mil", "true rng");
        }
        println!();
        for m in BDC {
            print!("{:5.0} {:6.2} |", m.range_yd, m.mil);
            for c in &curves {
                print!(
                    " {:>8.1} {:>7.2} {:>9} |",
                    c.miss_in(m),
                    c.miss_mil(m),
                    c.true_range(m).map_or("--".into(), |r| format!("{r:.0}"))
                );
            }
            println!();
        }
        for (i, (n, _)) in zeros.iter().enumerate() {
            let (reach, worst) = usable_to(&curves[i], TOL_IN);
            println!(
                "  {:<12} every mark within +/-{TOL_IN}\" out to {:.0} yd (worst {:.1}\") | RMS 300-800 {:.2} mil",
                n,
                reach,
                worst,
                rms_mil(&curves[i], &fit_marks())
            );
        }
        println!();
    }

    println!("=== What muzzle velocity is the BDC drawn for? (best RMS fit, 300-800) ===");
    for load in loads() {
        for (n, z) in [
            ("50 yd zero", zero_at(50.0, 0.0, sea)),
            ("100 yd zero", zero_at(100.0, 0.0, sea)),
        ] {
            let (mv, e) = best_mv(&load, &rifle, &z, sea);
            println!(
                "  {:<18} {:<12} best MV {:5.0} fps (RMS {:.2} mil)",
                load.proj.name, n, mv, e
            );
        }
    }
    println!();

    println!(
        "=== Design check: Mk262-class 77gr (G7 0.190). Manual: 50 yd zero; spec 2750 fps from 20\" ==="
    );
    {
        let mut mk262 = loads().pop().unwrap();
        mk262.mv = 2750.0;
        let z50 = zero_at(50.0, 0.0, sea);
        let z100 = zero_at(100.0, 0.0, sea);
        let (mv100, rms100) = best_mv(&mk262, &rifle, &z100, sea);
        let c50 = Curve::new(&mk262, &rifle, &z50, sea);
        let fit100 = Load {
            proj: mk262.proj.clone(),
            mv: mv100,
        };
        let c100 = Curve::new(&fit100, &rifle, &z100, sea);
        println!(
            "  Misses in mil. At 2750 the mark spacing is off by 0.45 mil from 300 to 800 (tilt); the spacing"
        );
        println!(
            "  fits ~2820-2860 fps, where a 50 yd zero leaves every mark ~0.3 mil off. A 100 yd zero at {mv100:.0} fps"
        );
        println!(
            "  fits every mark flat (RMS {rms100:.2}), so the marks alone don't confirm the stated design point."
        );
        println!(
            "  {:>4}  {:>5}  {:>20}  {:>24}",
            "yd",
            "mark",
            "50 yd zero @ 2750",
            format!("100 yd zero @ {mv100:.0}")
        );
        for m in BDC {
            println!(
                "  {:4.0}  {:5.2}  {:+9.2} mil {:+6.1}\"  {:+13.2} mil {:+6.1}\"",
                m.range_yd,
                m.mil,
                c50.miss_mil(m),
                c50.miss_in(m),
                c100.miss_mil(m),
                c100.miss_in(m)
            );
        }
        println!(
            "  RMS 300-800: 50 yd zero @ 2750 {:.2} mil | 100 yd zero @ {mv100:.0} {rms100:.2} mil",
            rms_mil(&c50, &fit_marks())
        );
    }
    println!();

    println!(
        "=== Primary Arms' own chart (manual p.7), rows for this barrel, zeroed at each altitude (ICAO) ==="
    );
    // (load, chart row, zero range, altitudes ft, inches high at the zero range)
    type ChartRow = (
        &'static str,
        &'static str,
        f64,
        &'static [f64],
        &'static [f64],
    );
    let pa: [ChartRow; 3] = [
        (
            "M193 55gr FMJBT",
            "14\" M193: zero at 50 yd at 1000/2000/3000 ft",
            50.0,
            &[1000.0, 2000.0, 3000.0],
            &[0.0, 0.0, 0.0],
        ),
        (
            "SS109/M855 62gr",
            "14.5\" M855: +1.0 / +0.5 / 0 inch at 100 yd at 1000/2000/3000 ft",
            100.0,
            &[1000.0, 2000.0, 3000.0],
            &[1.0, 0.5, 0.0],
        ),
        (
            "Hornady 77gr OTM",
            "77gr SMK, 2700-2750 fps: +1.0 inch at 100 yd (no altitude given; sea level here)",
            100.0,
            &[0.0],
            &[1.0],
        ),
    ];
    for load in loads() {
        let Some((_, label, zr, alts, offs)) = pa.iter().find(|p| p.0 == load.proj.name) else {
            continue;
        };
        println!("  {} @ {:.0} fps ({label})", load.proj.name, load.mv);
        for (alt, off) in alts.iter().zip(offs.iter()) {
            let atmo = Atmosphere::icao_at_altitude_ft(*alt);
            let c = Curve::new(&load, &rifle, &zero_at(*zr, *off, atmo), atmo);
            let (bo, reach) = best_offset(&load, &rifle, atmo, TOL_IN);
            print!("    {:>5.0} ft {:+.1}\" @{zr:.0}: miss in @", alt, off);
            for m in BDC
                .iter()
                .filter(|m| m.range_yd % 100.0 == 0.0 && m.range_yd >= 300.0)
            {
                print!(" {:.0}:{:+.1}", m.range_yd, c.miss_in(m));
            }
            let (r0, _) = usable_to(&c, TOL_IN);
            println!(
                "   | +/-{TOL_IN}\" to {r0:.0} yd; best here {bo:+.2}\" @100 -> {reach:.0} yd"
            );
        }
    }
    println!();

    println!("=== Wind: what each dot actually represents (full-value crosswind, mph) ===");
    println!(
        "Reticle: two dots per row for 5 and 10 mph (at 400 the hash end is 5 mph); the row numerals\n\
         4 / 6 / 8 sit at the 15 mph hold. Spin drift (right, RH twist) listed separately."
    );
    for load in loads() {
        let z = zero_at(100.0, 0.0, sea);
        let s = Solver::default();
        let angle = s.bore_angle(&load.proj, &rifle, load.mv, &z);
        let mut cond = Conditions::standard();
        cond.spin_drift = false;
        cond.aero_jump = false;
        cond.wind = Wind::crosswind_from_left(10.0);
        let ranges: Vec<f64> = WIND.iter().map(|w| w.0).collect();
        let wind = s.trajectory(&load.proj, &rifle, load.mv, angle, &cond, &ranges);
        let mut cond_sd = Conditions::standard();
        cond_sd.wind = Wind::crosswind_from_left(10.0);
        let full = s.trajectory(&load.proj, &rifle, load.mv, angle, &cond_sd, &ranges);
        println!("  {}", load.proj.name);
        println!(
            "    range | 10mph drift  | '5 mph' hold = | '10 mph' hold = | '15 mph' numeral = | spin drift | aero jump @10mph"
        );
        for ((w, p), f) in WIND.iter().zip(&wind).zip(&full) {
            let per_mph = p.windage_mil / 10.0;
            let w15 = WIND15
                .iter()
                .find(|n| n.0 == w.0)
                .map_or("--".into(), |n| format!("{:.1} mph", n.1 / per_mph));
            println!(
                "    {:5.0} | {:5.2} mil {:4.1}\" | {:5.1} mph     | {:5.1} mph       | {:>9}          | {:4.2} mil   | {:+.2} mil",
                w.0,
                p.windage_mil,
                p.windage_in,
                w.1 / per_mph,
                w.2 / per_mph,
                w15,
                f.spin_drift_in / (w.0 * 0.036),
                f.aero_jump_in / (w.0 * 0.036),
            );
        }
    }
    println!();

    println!(
        "=== Moving-target leads (dots on the horizontal: 3 / 6 / 9 mph at 1.70 / 3.30 / 5.00 mil) ==="
    );
    for load in loads() {
        let z = zero_at(100.0, 0.0, sea);
        let s = Solver::default();
        let angle = s.bore_angle(&load.proj, &rifle, load.mv, &z);
        let ranges = [100.0, 200.0, 300.0, 400.0, 500.0, 600.0];
        let pts = s.trajectory(
            &load.proj,
            &rifle,
            load.mv,
            angle,
            &Conditions::standard(),
            &ranges,
        );
        print!("  {:<18}", load.proj.name);
        for p in &pts {
            // Required lead for a target crossing at 3 mph, scaled to each dot's speed.
            let lead3 = ((3.0 * 5280.0 / 3600.0) * p.time_s / (p.range_yd * 3.0)).atan() * 1000.0;
            print!(
                " | {:.0}yd {:.2}/{:.2}/{:.2}",
                p.range_yd,
                lead3,
                lead3 * 2.0,
                lead3 * 3.0
            );
        }
        println!();
        let _ = LEADS;
    }
    println!();

    println!(
        "=== Sensitivity: miss (inches) at 500 / 600 / 800 with each load's best sea-level zero ==="
    );
    println!(
        "  Rows marked (re-zeroed) redo the zero under that condition, as you would with that ammo or\n\
         \x20 at that place. Rows marked (zeroed at 59 F) keep the sea-level zero. Air temperature only:\n\
         \x20 powder temperature also moves MV, which the +/-75 fps rows bracket."
    );
    for load in loads() {
        let (off, _) = best_offset(&load, &rifle, sea, TOL_IN);
        let run_zeroed = |label: &str, load: &Load, rifle: &Rifle, zero_atmo, atmo| {
            let c = Curve::new(load, rifle, &zero_at(100.0, off, zero_atmo), atmo);
            let pick = |r: f64| c.miss_in(BDC.iter().find(|m| m.range_yd == r).unwrap());
            println!(
                "    {:<26} {:+6.1} {:+6.1} {:+6.1}",
                label,
                pick(500.0),
                pick(600.0),
                pick(800.0)
            );
        };
        let run = |label: &str, load: &Load, rifle: &Rifle, atmo| {
            run_zeroed(label, load, rifle, atmo, atmo)
        };
        println!("  {} (zero {:+.2}\" @100)", load.proj.name, off);
        run("baseline", &load, &rifle, sea);
        for alt in [2500.0, 5000.0] {
            run(
                &format!("{alt:.0} ft (re-zeroed)"),
                &load,
                &rifle,
                Atmosphere::icao_at_altitude_ft(alt),
            );
        }
        for t in [20.0, 100.0] {
            run_zeroed(
                &format!("{t:.0} F (zeroed at 59 F)"),
                &load,
                &rifle,
                sea,
                sea.with_temp_f(t),
            );
        }
        for dv in [-75.0, 75.0] {
            let l = Load {
                proj: load.proj.clone(),
                mv: load.mv + dv,
            };
            run(&format!("MV {dv:+.0} fps (re-zeroed)"), &l, &rifle, sea);
        }
        for sh in [2.5, 3.1] {
            run(
                &format!("sight ht {sh}\" (re-zeroed)"),
                &load,
                &Rifle {
                    sight_height_in: sh,
                    ..rifle
                },
                sea,
            );
        }
    }
}

/// Per-mark misses for each load and zero, and the wind each dot represents.
fn write_csv(dir: &str, rifle: &Rifle, sea: Atmosphere) {
    use std::fmt::Write;
    let mut miss = String::from(
        "load,mv,zero,zero_offset_in_at_100,range_yd,mark_mil,miss_in,miss_mil,true_range_yd\n",
    );
    let mut wind = String::from(
        "load,mv,range_yd,drift_mil_per_10mph,dot5_mph,dot10_mph,numeral15_mph,spin_drift_mil\n",
    );
    for load in loads() {
        let (off, _) = best_offset(&load, rifle, sea, TOL_IN);
        for (name, z) in [
            ("50 yd zero", zero_at(50.0, 0.0, sea)),
            ("100 yd zero", zero_at(100.0, 0.0, sea)),
            (BEST, zero_at(100.0, off, sea)),
        ] {
            let c = Curve::new(&load, rifle, &z, sea);
            let zoff = if name == BEST { off } else { c.elev_in[100] };
            for m in BDC {
                writeln!(
                    miss,
                    "{},{},{},{:.2},{},{},{:.3},{:.4},{}",
                    load.proj.name,
                    load.mv,
                    name,
                    zoff,
                    m.range_yd,
                    m.mil,
                    c.miss_in(m),
                    c.miss_mil(m),
                    c.true_range(m).map_or(String::new(), |r| format!("{r:.1}"))
                )
                .unwrap();
            }
        }
        let s = Solver::default();
        let angle = s.bore_angle(&load.proj, rifle, load.mv, &zero_at(100.0, 0.0, sea));
        let mut cond = Conditions::standard();
        cond.aero_jump = false;
        cond.wind = Wind::crosswind_from_left(10.0);
        let ranges: Vec<f64> = WIND.iter().map(|w| w.0).collect();
        let pts = s.trajectory(&load.proj, rifle, load.mv, angle, &cond, &ranges);
        for (w, p) in WIND.iter().zip(&pts) {
            let sd_mil = p.spin_drift_in / (w.0 * 0.036);
            let per_mph = (p.windage_mil - sd_mil) / 10.0;
            let w15 = WIND15
                .iter()
                .find(|n| n.0 == w.0)
                .map_or(String::new(), |n| format!("{:.3}", n.1 / per_mph));
            writeln!(
                wind,
                "{},{},{},{:.4},{:.3},{:.3},{},{:.4}",
                load.proj.name,
                load.mv,
                w.0,
                per_mph * 10.0,
                w.1 / per_mph,
                w.2 / per_mph,
                w15,
                sd_mil
            )
            .unwrap();
        }
    }
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(format!("{dir}/miss.csv"), miss).unwrap();
    std::fs::write(format!("{dir}/wind.csv"), wind).unwrap();
    eprintln!("wrote {dir}/miss.csv and {dir}/wind.csv");
}
