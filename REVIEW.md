# Review notes

These are notes for whoever checks this work. Each section lists what was asked, what I claimed, the evidence for it, and how to re-check it. The last sections list the assumptions and the places I'm least confident.

**Fastest full check:** `cargo test --release && ./scripts/reproduce.sh`. All 18 tests should pass, and `extract_reticle.py` should end with `ALL MATCH`. The run takes about 35 s and writes everything to `out/`.

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
- **Precision:** the Gen 1 MIL figures in the image's "What changed" notes are pixel measurements off ~1000 px renders, so expect roughly ±0.05-0.1 MIL. The G2 BDC rows, wind holds, lead dots and ranging-bar positions come from the vector extraction in section 3.
- **Two errors in the first version, fixed in the current image** (tell the user if any older copy is floating around):
  - It called the "10" on the G2 bottom post a 1000 yd hold. It is a 10 MIL marker; the manual says the BDC ends at 800.
  - It called the dots beside the horseshoe "lead/wind" dots. The manual says they are 3/6/9 mph moving-target leads.

### Gen 2 vs Trijicon Credo 1-8x28 (added 2026-10-01)

- **Which Credo:** the 1-8x28 FFP **MRAD Segmented Circle** (CR828-C-2900032), picked because it is milliradian like the PLxC. A MOA Segmented Circle variant (CR828-C-2900055) also exists and is not shown.
- **Source:** Trijicon's official render `CR828-C-2900032-reticle.png` (1600x1600, from trijicon.com; sha256 `552c70b24b99a971009fa4caca2e3544bba2e97c2e3c79f32a984c801f080713`), scaled at 11.92 px/MRAD from its 5/10/15/20/25 MRAD long hashes.
- **Cross-checks of the scale:**
  - Trijicon's dimension sheet (`Credo_MRAD_Segmented_Circle_FFP_Reticle_2900032.pdf`, sha256 `632e0bd2d38d96c1c960526bcbfc9fd900b522cd56e1c07397941cc496097097`) is marked "not drawn to scale", so I use its numbers as checks only. Measured off the render: ring 19.30 MRAD across the segment ends (sheet 19.3), gaps 7.47 (7.5), about 1.5 thick (1.5). `credo_check()` in the script asserts this on every run.
  - In the overlay, the G2's 10 and 20 MIL post marks land on the Credo's 10 and 20 MRAD hashes. The two renders were scaled independently, each from its own markers.
  - The sheet's hash layout (0.5 MRAD from 1 to 5 MRAD, 1 MRAD beyond, crosshair starting 1 MRAD from center) matches the render.
- **Field of view:** the dashed circle is the Credo's 8x field of view from Trijicon's spec sheet (2.53°, about ±22 MRAD). The Credo's heavy posts start at 30 MRAD, so they are out of view at 8x. The PLxC's own circle in the G2 render is Primary Arms' drawing, not a measured field of view.
- **Derived numbers in the notes:** the Credo ring's outer diameter (~20.7 MRAD) follows from 19.3 across segment ends cut at ±3.75 MRAD. "1.66 MRAD per 10 mph at 500" is the solver's value for the 77gr at 2600 fps.

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

- **Source:** the vector art in Primary Arms' G2 reticle manual: PDF page 10 ("MILS", printed page 9) and PDF page 6 ("BDC AUTO RANGING").
  - Fetched from the Wayback Machine (`web.archive.org/web/2025id_/...`), because the live site is behind a bot check.
  - sha256 `cd3ae980a805f82e4254ebf8f66086b1d9cf4bc7704b2ea9d430ac041fc16535`.
- **Scale:** 13.609 drawing units per MIL, from the 1-MIL ticks on the side ranging scale. Independent checks against the manual's own labels and stated design intent:
  - The drawing's "6 MILS" and "1 MIL" dimension brackets measure 5.996 and 1.008 MIL, and the five ranging bars sit at 6.00-10.00 MIL from center (within 0.004).
  - The ranging bars are 5'10" at their labeled range to within 0.003 MIL (400-800 yd).
  - The BDC hash widths are 18" at their range to within 0.002 MIL.
  - The 5 and 10 mph wind dots scale exactly 1:2 on every row, and the row numerals sit at exactly 3x the 5 mph hold.
- **How the marks were read:**
  - All drawn marks match `src/reticle.rs` to within 0.003 MIL: the 350-800 hashes, the 300 yd stem tip, the wind dots, the row numerals and the lead dots.
  - 100 and 200 yd have no physical mark. They come from leader lines in a schematic figure, so I'd trust them only to about ±0.04 MIL; the script uses a 0.06 MIL tolerance for them. 200 yd is excluded from every fit.
  - During review prep I changed 200 yd from 0.36 to 0.34 to match the scripted extraction. My original hand estimate had assumed a 0.03 leader bias that the scripted fit shows is about 0. No headline number changed.
- **Interpretation calls a reviewer should look at:**
  - **What the wind holds mean.** The manual's text says each BDC row has "2 dots on each side, representing drift from 5 mph and 10 mph". Its page-8 figure has three leaders labeled 5/10/15 mph: the 5 and 10 mph leaders run through the first and second dot columns, and the 15 mph leader runs through the row numerals "4", "6", "8". The geometry agrees: dot 2 / dot 1 is exactly 2.0 on every row, and the numeral centers sit at exactly 3x the 5 mph hold (1.86 / 3.14 / 4.71 MIL vs 1.88 / 3.14 / 4.71). So the numerals are 15 mph holds, on the 400/600/800 rows only (`WIND15` in `src/reticle.rs`). The first version of this repo missed them.
  - **The 400 yd row** has only one dot each side (1.250 MIL). The hash's own end (0.626, which is also the 18" width) is the 5 mph hold. Its 1:2 ratio to the dot and the "4" numeral at 3x (1.86) both confirm it, and the page-8 figure's 5 mph leader points at it.
  - **Design point.** The manual says Mk262 (77gr, Mk262 spec 2750 fps from 20") with a 50 yd zero. With a G7 0.190 bullet at 2750 the marks fit 400-600 within 0.10 MIL. But the mark spacing is off by 0.45 MIL between 300 and 800: the 300 mark is ~0.2 MIL low of the bullet and the 800 mark ~0.2 MIL high. The spacing fits ~2820-2860 fps, and at those speeds a 50 yd zero leaves every mark ~0.3 MIL off. A 100 yd zero at ~2860 fps fits every 300-800 mark within 0.13 MIL (RMS 0.07). Changing the zero shifts all marks by a near-constant angle, while velocity changes the spacing, so these are separable. G1 0.372 behaves the same as G7 here. Tests: `bdc_matches_mk262_with_50_yd_zero_through_600`, `no_50_yd_zero_fits_every_mark_tightly`, `bdc_fits_77gr_with_100_yd_zero_flat`. The marks support the extraction, but they don't confirm the manual's stated design point, and they don't validate the solver on their own (the py-ballisticcalc cross-check does that).
  - **A unit slip in Primary Arms' manual (MILS page):** it gives "RANGE (YARDS) = Target Size (Inches) * 25.4 / Target MILs". Inches × 25.4 / mils gives **meters**, which is 8.6% short of yards. The yard formula is inches × 27.78 / mils. Not used anywhere in this repo.
- **Primary Arms' 77gr line doesn't fit either.** The chart says 77gr SMK at 2700-2750 fps, +1.0" at 100. Under this model that lands 0.4-0.8 MIL low at 800, so PA's own assumptions (sight height, BC, atmosphere) differ from these and aren't known.

## 4. BDC calibration (`src/bin/bdc.rs` → `out/bdc_report.txt`, `out/bdc_calibration.png`)

**Headline results** (14.5" barrel, ICAO sea level, 2.75" sight height, 1:7 twist):

| Load | 50 yd zero | 100 yd zero | Best zero (every mark within ±6") | "10 mph" dot actually equals | "15 mph" numeral actually equals |
|---|---|---|---|---|---|
| M193 @ 3032 | good to 500 yd | 400 yd | +2.05" @100 → 550 yd | ~6.4-7.1 mph | ~9.6-10.5 mph |
| SS109 @ 2808 | 500 yd | 350 yd | +2.65" @100 → 600 yd | ~7.9-8.4 mph | ~11.8-12.5 mph |
| 77gr @ 2600 | 450 yd | 300 yd | +3.20" @100 → 600 yd | ~9.7-10.0 mph | ~14.6-14.8 mph |

**Things to check:**
- **"Miss" at a mark** means: hold the mark on a target at the mark's range, then measure impact relative to the target. That equals `elev_in(R) + R·36·tan(mark_mil/1000)`.
- **The "best zero" criterion is my choice:** the longest reach with every mark within ±6", tie-broken by the smallest worst miss. An earlier version used RMS over 300-800 yd. That let 700-800 yd drag the zero 9-13" high at 300-450, so I replaced it, and the user only saw the ±6" version. The RMS figures are still printed for reference.
- **Primary Arms' chart comparison:** the report evaluates PA's chart rows (manual PDF page 7) for this barrel length, in an ICAO standard atmosphere at each altitude, zeroed there:
  - **M193, 14" row:** zero at 50 yd, no offset, at 1000/2000/3000 ft. With 3032 fps every mark is within ±6" to 550 / 550 / 600 yd, the same reach as the optimized best zero. PA's guidance works for this barrel.
  - **M855, 14.5" row:** +1.0 / +0.5 / 0" at 100 yd. Reaches 450 yd at all three altitudes; the optimized zero there reaches 600-650.
  - **77gr SMK row:** +1.0" at 100 (for 2700-2750 fps; no altitude given, sea level used). At 2600 fps it reaches only 400 yd.
  - The first version compared against the 16" rows, which don't match this rifle.
- **Sensitivity block:** shows altitude, temperature, ±75 fps velocity and 2.5/3.1" sight height. Altitude, velocity and sight-height rows re-zero under that condition. Temperature rows keep the 59 °F zero and change only air temperature; powder temperature also moves velocity, which the ±75 fps rows bracket. Velocity and air density dominate; sight height barely matters.

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

**77gr velocity:** 2600 fps in both studies, a guess for the user's 14.5" barrel. Study 2 originally used 2650 (a 16" guess); it was aligned in the review pass.

## Assumptions and sources

| Input | Value | Source / confidence |
|---|---|---|
| M193 G7 BC | 0.120 | Commonly cited Litz measurement (secondary source). Medium. A single G7 BC is weaker past ~650 yd, where M193 goes transonic. |
| M855 G7 BC | 0.151 | Litz (secondary). Medium-high. |
| Hornady 77gr OTM | G7 0.190 | Hornady AeroMatch 77 (22777) as listed by retailers. The user's "77gr OTM" is assumed to be this bullet, which Black Hills loads in its Mk262-type ammo. |
| Bullet lengths | 0.750 / 0.906 / 0.990" | Typical values, used only for stability → spin drift and aero jump. Miller's rule is poor for M855's steel penetrator. |
| Barrel | 14.5" | The user's rifle (from the KL request). Picks Primary Arms' chart rows. |
| Sight height | 2.75" | AR rail (~1.2" above bore) plus a ~1.54" LPVO mount. Not measured on the user's rifle. |
| Twist | 1:7 RH | Assumed. |
| Atmosphere | ICAO sea level, dry | Altitude and temperature sensitivity shown separately. |
| M193 / SS109 velocity | 3032 / 2808 fps | From the user. |
| 77gr velocity | 2600 (both studies) | My guess for 14.5": Mk262 is 2750 from 20". |

## Least-confident items, for the reviewer to focus on

1. **Litz aero-jump and spin-drift formulas and sign** (`src/stability.rs`), written from memory. They're small effects, and the elevation calibration doesn't use them.
2. **G7 BCs** come from secondary sources, and M193 beyond 650 yd is transonic, where one G7 BC is a rough fit.
3. **The design point** (section 3): the marks fit a 100 yd zero at ~2860 fps better than the manual's 50 yd zero, and PA's own 77gr chart line doesn't fit this model. Unexplained.
4. **The 100/200 yd hold positions** come from a schematic figure.
5. **The k-NN estimator implementation** (`kl.py: knn_kl`). Check the `log(m/(n-1))` term and the self-exclusion (`k+1`).
6. **Interpolation:** `interp` in `src/solver.rs` uses Hermite interpolation in time, with an approximate derivative in the Newton solve. The converged point is correct regardless, and the tests cover it indirectly.

## Not done

- No CI.
- Python dependencies aren't locked; `uv run --with` resolves current versions (the versions used are listed in README).
- No 6-DOF model. Coriolis isn't exercised in any study.
- `traj` panics, rather than returning an error, on bad arguments or a range beyond the bullet's flight time.

## Changes after the 2026-09-30 review

- Added the 15 mph wind holds (row numerals) to `src/reticle.rs`, the extraction script, the wind report, `wind.csv`, the chart and the tests.
- The extraction now also checks the scale against the manual's "6 MILS" / "1 MIL" brackets and the ranging-bar positions.
- Primary Arms' chart comparison now uses the 14"/14.5" rows (M193 at a 50 yd zero) and PA's 77gr SMK line, not the 16" rows.
- 77gr velocity aligned to 2600 fps in study 2 (was 2650).
- Replaced the loose 0.25 MIL design-point test with tests that pin down the tilt, the 50 vs 100 yd zero fit, and the numerals.
- Sensitivity rows now say whether they re-zero; temperature rows keep the 59 °F zero.
- Regenerated `out/`, both images (also on the Desktop), and the KL page (its inputs didn't change).
- 2026-10-01: added the Gen 2 vs Trijicon Credo 1-8x28 MRAD section to the comparison image (section 1).
