"""Reference trajectories from py-ballisticcalc, for tests/reference.rs.

    uv run --with py-ballisticcalc==2.3.1 scripts/xcheck_pbc.py

Same inputs as the Rust test: G7, 2.75" sight height, 100 yd zero, ICAO sea level,
10 mph wind from 9 o'clock, twist 0 (no spin drift). Compare with:

    cargo run --release --bin traj -- --mass 55 --len 0.75 --bc 0.120 --mv 3032 \
        --zero 100 --wind 10 --wind-clock 9 --no-spin --no-jump --step 100
"""
from py_ballisticcalc import *
from py_ballisticcalc.unit import *

PreferredUnits.distance = Unit.Yard
CASES = [
    ("M193", 55, 0.750, 0.120, 3032),
    ("M855", 62, 0.906, 0.151, 2808),
    ("77gr OTM", 77, 0.990, 0.190, 2650),
]

for name, gr, length, bc, mv in CASES:
    dm = DragModel(bc, TableG7, weight=Weight.Grain(gr), diameter=Distance.Inch(0.224), length=Distance.Inch(length))
    ammo = Ammo(dm, mv=Velocity.FPS(mv))
    weapon = Weapon(sight_height=Distance.Inch(2.75), twist=Distance.Inch(0))
    calc = Calculator()
    calc.set_weapon_zero(Shot(weapon=weapon, ammo=ammo, atmo=Atmo.icao()), Distance.Yard(100))
    shot = Shot(weapon=weapon, ammo=ammo, atmo=Atmo.icao(), winds=[Wind(Velocity.MPH(10), Angular.OClock(9))])
    res = calc.fire(shot, trajectory_range=Distance.Yard(800), trajectory_step=Distance.Yard(100))
    print(name)
    for p in res.trajectory:
        print(
            f"  {p.distance >> Distance.Yard:4.0f} yd  t={p.time:.4f}  v={p.velocity >> Velocity.FPS:7.1f}"
            f"  elev_in={p.height >> Distance.Inch:9.3f}  wind_in={p.windage >> Distance.Inch:8.3f}"
        )
