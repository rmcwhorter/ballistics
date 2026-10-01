# Review notes

These are notes for whoever checks this work. Each section lists what was asked, what I claimed, the evidence for it, and how to re-check it. The last sections list the assumptions and the places I'm least confident.

**Fastest full check:** `cargo test --release && ./scripts/reproduce.sh`. All 16 tests should pass, and `extract_reticle.py` should end with `ALL MATCH`. The run takes about 35 s and writes everything to `out/`.

## What the user asked for, in order

1. An image of the old-generation PLxC 1-8 BDC reticle (ACSS Raptor M8 Yard) next to the new RDB one (ACSS Raptor 5.56/.308 Yard G2), to see what changed.
2. "The best possible ballistic calculator". Use it to show how M193 @ 3032 fps, SS109 @ 2808 fps and Hornady 77gr OTM ("whatever is plausible") calibrate to the **G2** BDC at a 50 or 100 yd zero, **including the wind holds**.
3. A way to learn KL divergence from M193 vs 77gr trajectories "out of this 14.5" barrel".

Deliverables outside the repo:
- `~/Desktop/plxc_raptor_gen1_vs_g2.png` and `~/Desktop/plxc_g2_bdc_calibration.png`. Both are regenerated into `out/`.
- The published page https://claude.ai/artifact/N5kzVgCwGVyvAbiCG2FtLE (private to the user). It is built from `site/template.html` and `out/kl.json`.

## 1. Gen 1 vs Gen 2 image (`scripts/reticle_compare.py`)

- **Source:** Primary Arms' raster renders from their PLx reticle guide. They're drawn at different scales, so both are resampled to the same px/MIL. The scale for each comes from that render's own MIL markers; the numbers are in the script's docstring.
- **Cross-check of the scale:** after scaling, the 400-800 yd BDC rows of both reticles agree to about 0.05 MIL. The G2 rows also match the vector extraction in section 3.
- **Precision:** the MIL figures in the image's "What changed" notes are pixel measurements off ~1000 px renders, so expect roughly ±0.05-0.1 MIL.
- **Two errors in the first version, fixed in the current image** (tell the user if any older copy is floating around):
  - It called the "10" on the G2 bottom post a 1000 yd hold. It is a 10 MIL marker; the manual says the BDC ends at 800.
  - It called the dots beside the horseshoe "lead/wind" dots. The manual says they are 3/6/9 mph moving-target leads.

## 2. Solver (`src/`)

- **Model:** 3-DOF point mass with RK4, dt = 50 µs. Drag is BC-scaled G1/G7 (standard tables, PCHIP-interpolated), with real air density.
- **Additional effects:**
  - Moist air (Buck vapor pressure, virtual temperature for the speed of sound).
  - 3-D wind and look angle.
  - Coriolis, as -2Ω×v rotated into the line-of-sight frame.
  - Zeroing by Newton iteration on bore angle.
  - Litz spin drift and crosswind aerodynamic jump, added after integration.
- **Evidence it's right:**
  - `tests/reference.rs` matches py-ballisticcalc 2.3.1 (an independent RK4 solver) within 0.1" + 0.02% of drop, 0.05" of drift, 2 ms of time of flight and 1 fps. That's for M193/M855/77gr out to 800 yd, with 10 mph wind. The largest observed gap is 0.06" of drop at 800 yd (M193).
  - Unit tests cover the zero landing, step-size convergence (dt vs dt/2 within 1e-4" at 800 yd), the wind and aero-jump directions, Coriolis direction, the ICAO table, and drag-table interpolation.
- **Not independently validated:**
  - **Coriolis:** sign test only. None of the studies use it.
  - **Spin drift and aero jump:** these are Litz's empirical formulas, entered from memory (`src/stability.rs`) and not checked against the book. Aero-jump sign convention: with right-hand twist, wind from the left pushes the bullet up. The size is ≤0.18 MIL per 10 mph here. Neither affects the elevation calibration numbers, because `bdc` runs in still air.

## 3. Reticle subtensions (`src/reticle.rs`, `scripts/extract_reticle.py`)

- **Source:** the vector art in Primary Arms' G2 reticle manual: page 10 ("MILS") and page 6 ("BDC AUTO RANGING").
  - Fetched from the Wayback Machine (`web.archive.org/web/2025id_/...`), because the live site is behind a bot check.
  - sha256 `cd3ae980a805f82e4254ebf8f66086b1d9cf4bc7704b2ea9d430ac041fc16535`.
- **Scale:** 13.609 drawing units per MIL, from the 1-MIL ticks on the side ranging scale. Three independent checks against design intent the manual states:
  - The ranging bars are 5'10" at their labeled range to within 0.003 MIL (400-800 yd).
  - The BDC hash widths are 18" at their range to within 0.002 MIL.
  - The 5 and 10 mph wind dots scale exactly 1:2 on every row.
- **How the marks were read:**
  - All drawn marks match `src/reticle.rs` to within 0.003 MIL: the 350-800 hashes, the 300 yd stem tip, the wind dots and the lead dots.
  - 100 and 200 yd have no physical mark. They come from leader lines in a schematic figure, so I'd trust them only to about ±0.04 MIL; the script uses a 0.06 MIL tolerance for them. 200 yd is excluded from every fit.
  - During review prep I changed 200 yd from 0.36 to 0.34 to match the scripted extraction. My original hand estimate had assumed a 0.03 leader bias that the scripted fit shows is about 0. No headline number changed.
- **Interpretation calls a reviewer should look at:**
  - **What the wind dots mean.** The manual's text says each BDC row has "2 dots on each side, representing drift from 5 mph and 10 mph". Its page-8 figure has three arrows labeled 5/10/15 mph. I used the text. If the hash end, dot 1 and dot 2 were 5/10/15 mph they would scale 1:2:3, but the measured ratios are nowhere near that (500 yd: 0.50 / 0.81 / 1.66), while dot 2 / dot 1 is exactly 2.0. The 77gr check in `tests/reference.rs` also lands on 5.0/10.2 mph.
  - **The 400 yd row** has only one dot each side (1.250 MIL). I treated the hash's own end (0.626, which is the 18" width) as the 5 mph hold. That is consistent with the 1:2 ratio.
  - **Design point:** a 77gr G7 0.190 bullet at 2750 fps (Mk262 spec) with a 50 yd zero lands within 0.25 MIL of every mark from 300 to 800 yd (test `bdc_is_drawn_for_mk262_with_50_yd_zero`). The best-fit velocity is 2765 fps. This supports both the extraction and the solver.

## 4. BDC calibration (`src/bin/bdc.rs` → `out/bdc_report.txt`, `out/bdc_calibration.png`)

**Headline results** (ICAO sea level, 2.75" sight height, 1:7 twist):

| Load | 50 yd zero | 100 yd zero | Best zero (every mark within ±6") | "10 mph" dot actually equals |
|---|---|---|---|---|
| M193 @ 3032 | good to 500 yd | 400 yd | +2.05" @100 → 550 yd | ~6.4-7.1 mph |
| SS109 @ 2808 | 500 yd | 350 yd | +2.65" @100 → 600 yd | ~7.9-8.4 mph |
| 77gr @ 2650 | 450 yd | 350 yd | +3.00" @100 → 650 yd | ~10.1-10.3 mph |

**Things to check:**
- **"Miss" at a mark** means: hold the mark on a target at the mark's range, then measure impact relative to the target. That equals `elev_in(R) + R·36·tan(mark_mil/1000)`.
- **The "best zero" criterion is my choice:** the longest reach with every mark within ±6", tie-broken by the smallest worst miss. An earlier version used RMS over 300-800 yd. That let 700-800 yd drag the zero 9-13" high at 300-450, so I replaced it, and the user only saw the ±6" version. The RMS figures are still printed for reference.
- **Primary Arms' chart comparison:** the report also evaluates Primary Arms' own 100 yd zero offsets for 16" M193/M855 at 1000/2000/3000 ft, using an ICAO standard atmosphere at each altitude.
- **Sensitivity block:** shows altitude, temperature, ±75 fps velocity and 2.5/3.1" sight height. Velocity and air density dominate; sight height barely matters.

## 5. KL divergence study (`src/bin/sensitivity.rs`, `scripts/kl.py`, `site/`)

**Model:**
- 14.5" barrel: M193 3032 ± 35 fps, 77gr 2600 ± 15 fps.
- Dispersion: 0.75 / 0.35 MOA per axis.
- Shared wind-call error N(0, 3 mph).
- 50 yd zero; hold the BDC mark for the range.
- The noise parameters are illustrative, not measured, and the page says so.

**How the samples are drawn:** for each load, the solver tabulates impact against muzzle velocity (±5 SD grid). Drift and jump are reported per mph, because wind effects are linear to <0.01%, which I checked at 5/10/20 mph. Samples then draw velocity, wind and dispersion independently.

**How the exact KL is computed:**
- Given a velocity, a shot is exactly Gaussian, so the true density is a Gaussian mixture over velocity.
- `exact_logpdf` integrates that mixture with 48-node Gauss-Hermite quadrature.
- KL = mean of the log ratio over 50k samples.
- Caveat: the outer quadrature nodes (out to ±12.7 SD) sit beyond the ±5 SD grid, and `np.interp` clamps them to the grid edge. Those nodes carry 3e-7 of the total weight, so the effect is negligible but nonzero.

**Claims and their evidence:**
- **Exact vs Gaussian KL:** they agree within about 3% from 100 to 700 yd (worst: 2.6% at 500 yd). At 800 yd they diverge (50.4 vs 42.6 bits), because M193 goes transonic and its cloud stops being Gaussian. `kl.py` prints both.
- **k-NN estimator bias:** the Wang-Kulkarni-Verdú estimator (k = 5) is biased low and converges slowly. On true Gaussians with known KL it gives 12.4 bits → 4.1 / 5.8 / 7.1 at n = 2k / 20k / 200k (`scripts/knn_bias_check.py`).
- **Raw histograms give ∞:** some 1" bins hold M193 holes but no 77gr holes. The smoothed version (+0.5 count per bin) depends on that choice.
- **The "paradox":** at 100 yd, D(M193‖77gr) = 2.8 bits vs 1.0 the other way, yet single-shot identification is 65% (firing M193) vs 85% (firing 77gr). Identification uses exact densities and equal priors.
- **Stein's lemma wording:** "if you only rarely allow mistaking M193 for 77gr, the best chance of mistaking 77gr for M193 falls like 2^(-n·D(M193‖77gr))". I had the direction backwards in a draft and fixed it before the page went out. Worth a second look.
- **Fitting directions:** "Maximum likelihood is the first one" refers to minimizing D(data‖model).

**Velocity inconsistency:** the 77gr is 2650 fps in study 2 (a guess for a 16" barrel) but 2600 fps in study 3 (the user said 14.5"). Both are assumptions, flagged to the user.

## Assumptions and sources

| Input | Value | Source / confidence |
|---|---|---|
| M193 G7 BC | 0.120 | Commonly cited Litz measurement (secondary source). Medium. A single G7 BC is weaker past ~650 yd, where M193 goes transonic. |
| M855 G7 BC | 0.151 | Litz (secondary). Medium-high. |
| Hornady 77gr OTM | G7 0.190 | Hornady AeroMatch 77 (22777) as listed by retailers. The user's "77gr OTM" is assumed to be this bullet, which Black Hills loads in its Mk262-type ammo. |
| Bullet lengths | 0.750 / 0.906 / 0.990" | Typical values, used only for stability → spin drift and aero jump. Miller's rule is poor for M855's steel penetrator. |
| Sight height | 2.75" | AR rail (~1.2" above bore) plus a ~1.54" LPVO mount. Not measured on the user's rifle. |
| Twist | 1:7 RH | Assumed. |
| Atmosphere | ICAO sea level, dry | Altitude and temperature sensitivity shown separately. |
| M193 / SS109 velocity | 3032 / 2808 fps | From the user. |
| 77gr velocity | 2650 (study 2), 2600 (study 3) | My guesses: Mk262 is 2750 from 20". |

## Least-confident items, for the reviewer to focus on

1. **Litz aero-jump and spin-drift formulas and sign** (`src/stability.rs`), written from memory. They're small effects, and the elevation calibration doesn't use them.
2. **G7 BCs** come from secondary sources, and M193 beyond 650 yd is transonic, where one G7 BC is a rough fit.
3. **Wind-dot meaning** at 400 yd and the 5/10 vs 5/10/15 reading (see section 3).
4. **The 100/200 yd hold positions** come from a schematic figure.
5. **The k-NN estimator implementation** (`kl.py: knn_kl`). Check the `log(m/(n-1))` term and the self-exclusion (`k+1`).
6. **Interpolation:** `interp` in `src/solver.rs` uses Hermite interpolation in time, with an approximate derivative in the Newton solve. The converged point is correct regardless, and the tests cover it indirectly.

## Not done

- No CI.
- Python dependencies aren't locked; `uv run --with` resolves current versions (the versions used are listed in README).
- No 6-DOF model. Coriolis isn't exercised in any study.
- `traj` panics, rather than returning an error, on bad arguments or a range beyond the bullet's flight time.
