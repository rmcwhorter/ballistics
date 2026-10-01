#!/usr/bin/env bash
# Regenerate every result in this repo from scratch. Needs: cargo, uv, curl, pdftocairo
# (poppler), and ImageMagick for the Gen 1/Gen 2 image. Outputs land in out/ and site/.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p out

echo "== tests (unit + py-ballisticcalc reference + reticle design point)"
cargo test --release -q

echo "== reticle subtensions re-derived from the manual vs src/reticle.rs"
uv run -q --with svgelements scripts/extract_reticle.py

echo "== BDC calibration report"
cargo run --release -q --bin bdc | tee out/bdc_report.txt
cargo run --release -q --bin bdc -- --csv out
uv run -q --with matplotlib --with pandas scripts/plot.py out

echo "== KL divergence study"
cargo run --release -q --bin sensitivity > out/sensitivity.csv
uv run -q --with numpy --with scipy --with pandas scripts/kl.py out
uv run -q --with numpy --with scipy --with pandas scripts/knn_bias_check.py out
python3 scripts/build_page.py out

echo "== Reticle comparison image (Gen 1 vs Gen 2, Gen 2 vs Trijicon Credo)"
uv run -q --with numpy --with pillow scripts/reticle_compare.py

echo "== optional: independent solver reference values (compare with tests/reference.rs)"
echo "   uv run --with py-ballisticcalc==2.3.1 scripts/xcheck_pbc.py"
