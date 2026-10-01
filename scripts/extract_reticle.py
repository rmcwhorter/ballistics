"""Re-derive the G2 reticle subtensions in src/reticle.rs from Primary Arms' manual.

    uv run --with svgelements scripts/extract_reticle.py

Needs `curl` and poppler's `pdftocairo`. Downloads the manual (Wayback copy; the live
primaryarmsoptics.com URL is behind a bot check) into data/, converts the vector reticle
art on PDF page 10 ("MILS", printed page 9) and page 6 ("BDC AUTO RANGING") to SVG, decodes the geometry, and
compares every value against the constants in src/reticle.rs. Exits non-zero on mismatch.
"""
import hashlib
import re
import subprocess
import sys
from pathlib import Path

from svgelements import SVG, Path as SvgPath, Shape

ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / "data"
PDF_URL = "https://primaryarmsoptics.com/wp-content/uploads/2025/02/PLxC-1-8x24-FFP-RDB-RAPTOR-RETICLE-MANUAL_WEB.pdf"
PDF = DATA / "PLxC-1-8x24-FFP-RDB-RAPTOR-RETICLE-MANUAL_WEB.pdf"
TOL = 0.003  # MIL, for marks drawn as geometry
TOL_LEADER = 0.06  # MIL, for 100/200/300, read off leader lines in a schematic figure


def fetch():
    DATA.mkdir(exist_ok=True)
    if not PDF.exists():
        subprocess.run(
            ["curl", "-sSL", "-m", "120", "-A", "Mozilla/5.0", "-o", str(PDF),
             f"https://web.archive.org/web/2025id_/{PDF_URL}"],
            check=True,
        )
    print(f"manual sha256 {hashlib.sha256(PDF.read_bytes()).hexdigest()}")


def shapes(page):
    svg_path = DATA / f"manual-p{page}.svg"
    subprocess.run(["pdftocairo", "-svg", "-f", str(page), "-l", str(page), str(PDF), str(svg_path)], check=True)
    out = []
    for e in SVG.parse(str(svg_path)).elements():
        if not isinstance(e, Shape):
            continue
        bb = e.bbox()
        if bb is None:
            continue
        fill = str(e.fill) if e.fill is not None and e.fill.value is not None else None
        stroke = str(e.stroke) if e.stroke is not None and e.stroke.value is not None else None
        out.append(dict(e=e, bb=bb, fill=fill, stroke=stroke, w=bb[2] - bb[0], h=bb[3] - bb[1]))
    return out


def outline(shape):
    return [(s.end.x, s.end.y) for s in SvgPath(shape["e"]).segments() if s.end is not None]


def group(sorted_vals, gap=0.05):
    """Cluster sorted values whose neighbors are within `gap` (left/right twins)."""
    out = []
    for v in sorted_vals:
        if out and v - out[-1][-1] < gap:
            out[-1].append(v)
        else:
            out.append([v])
    return out


def runs(points, keep):
    """Contiguous runs (in outline order) of points satisfying `keep`."""
    groups, cur = [], []
    for p in points:
        if keep(p):
            cur.append(p)
        elif cur:
            groups.append(cur)
            cur = []
    if cur:
        groups.append(cur)
    return groups


def decode_mils_page():
    s = shapes(10)
    red = [x for x in s if x["fill"] == "#ff0000"]
    horseshoe = max(red, key=lambda x: x["w"])
    chevron = min(red, key=lambda x: x["w"])
    cx = (horseshoe["bb"][0] + horseshoe["bb"][2]) / 2
    black = [x for x in s if x["fill"] == "#000000"]

    # Side ranging block: 1-MIL ticks stick out to the left of its outer vertical line.
    block = max((x for x in black if x["bb"][2] < cx - 50 and 60 < x["w"] < 100), key=lambda x: x["h"])
    pts = outline(block)
    xmin = min(p[0] for p in pts)
    ys = sorted(p[1] for p in pts if abs(p[0] - xmin) < 1e-6)
    pairs = [(ys[i] + ys[i + 1]) / 2 for i in range(0, len(ys) - 1, 2)]
    spacing = [b - a for a, b in zip(pairs, pairs[1:])]
    scale = sorted(spacing)[len(spacing) // 2]  # median: the last tick is drawn thinner
    y0 = min(pairs, key=lambda y: abs(y - chevron["bb"][1]))
    print(f"scale {scale:.4f} units/MIL from {len(pairs)} ticks; crosshair y {y0:.3f}; chevron tip y {chevron['bb'][1]:.3f}")
    mil = lambda y: (y - y0) / scale

    # Vertical ranging bars: each is 5'10" (70 in) tall at its labeled range, so the half
    # above the crosshair is 35 in. The outermost (1-MIL scale) line is not a ranging bar.
    tops = sorted(-mil(min(p[1] for p in r)) for r in runs(pts, lambda p: p[1] < y0 - 5 and p[0] > xmin + 3))
    halves = [g[0] for g in group(tops, 0.01)][:-1]  # last is the 1-MIL scale line
    print("ranging bars, half-height above crosshair vs 35 in at range:")
    for r, h in zip([800, 700, 600, 500, 400], halves):
        print(f"      {r} yd bar {h:.3f} MIL vs {35 / (r * 0.036):.3f}")

    # Horizontal position of the ranging bars (each drawn as a pair of outline edges), and
    # the manual's own "6 MILS" / "1 MIL" dimension brackets (blue strokes).
    xs = sorted({round((p[0] - cx) / scale, 4) for p in pts if p[0] > xmin + 1e-6})
    bar_x = [-(a + b) / 2 for a, b in zip(xs[0::2], xs[1::2])][1:]  # first pair is the 1-MIL scale line
    brackets = sorted((x["w"] / scale for x in s if x["stroke"] == "#00aeef" and x["fill"] is None and x["w"] > 0.5
                       and x["bb"][2] < cx + 1), reverse=True)
    print("ranging bars, MIL from center (expect 10..6): " + " ".join(f"{b:.3f}" for b in bar_x))
    print(f"dimension brackets: '6 MILS' {brackets[0]:.3f}, '1 MIL' {brackets[1]:.3f}")

    # BDC stem: hashes are the runs of the outline that leave the stem to the right.
    stem = max((x for x in black if abs((x["bb"][0] + x["bb"][2]) / 2 - cx) < 1 and x["bb"][1] > y0), key=lambda x: x["h"])
    stem_pts = outline(stem)
    hashes = []
    for r in runs(stem_pts, lambda p: p[0] > cx + 0.9):
        ylo, yhi = min(p[1] for p in r), max(p[1] for p in r)
        hashes.append((mil((ylo + yhi) / 2), (max(p[0] for p in r) - cx) / scale))
    hashes.sort()
    stem_top = mil(stem["bb"][1])
    stem_bottom = mil(stem["bb"][3])

    # Dots: small filled circles. BDC wind dots sit on hash rows; lead dots on the crosshair.
    dots = [x for x in black if 2.8 < x["w"] < 6.5 and abs(x["w"] - x["h"]) < 0.2]
    dot_c = [(((d["bb"][0] + d["bb"][2]) / 2 - cx) / scale, mil((d["bb"][1] + d["bb"][3]) / 2)) for d in dots]
    lead_x = sorted(abs(dx) for dx, dy in dot_c if abs(dy) < 0.05)
    leads = [sum(g) / len(g) for g in group(lead_x)]
    rows = {}
    for dx, dy in dot_c:
        if dy > 0.5:
            rows.setdefault(round(dy, 1), []).append(abs(dx))

    # Row numerals "4" / "6" / "8" beside the BDC (glyph outlines ~0.4 x 0.58 MIL).
    numerals = {}
    for x in black:
        w, h = x["w"] / scale, x["h"] / scale
        dy = mil((x["bb"][1] + x["bb"][3]) / 2)
        if 0.3 < w < 0.5 and 0.5 < h < 0.65 and dy > 1:
            numerals.setdefault(round(dy, 1), []).append(abs((x["bb"][0] + x["bb"][2]) / 2 - cx) / scale)
    numerals = {k: sum(v) / len(v) for k, v in numerals.items()}
    return scale, y0, stem_top, stem_bottom, hashes, rows, leads, numerals, bar_x, brackets


def decode_bdc_page(stem_top_mil, stem_bottom_mil):
    """Map page 6 leader lines (100..800) into MIL using the stem's ends as anchors."""
    s = shapes(6)
    red = [x for x in s if x["fill"] == "#ff0000"]
    cx = sum(max(red, key=lambda x: x["w"])["bb"][0::2]) / 2
    stem = max((x for x in s if x["fill"] == "#000000" and abs((x["bb"][0] + x["bb"][2]) / 2 - cx) < 1.5), key=lambda x: x["h"])
    top, bot = stem["bb"][1], stem["bb"][3]
    leaders = sorted(x["bb"][3] for x in s if x["stroke"] == "#00aeef" and x["h"] < 3)
    to_mil = lambda y: stem_top_mil + (y - top) / (bot - top) * (stem_bottom_mil - stem_top_mil)
    return [to_mil(y) for y in leaders]


def rust_constants():
    src = (ROOT / "src" / "reticle.rs").read_text()
    bdc = {int(float(r)): float(m) for r, m in re.findall(r"range_yd:\s*([\d.]+),\s*mil:\s*([\d.]+)", src)}
    wind = {int(float(r)): (float(a), float(b)) for r, a, b in re.findall(r"\(([\d.]+),\s*([\d.]+),\s*([\d.]+)\),", src)}
    leads = [float(m) for m in re.findall(r"\([\d.]+,\s*([\d.]+),\s*\"\w+\"\)", src)]
    w15 = re.search(r"WIND15[^=]*=\s*&\[(.*?)\];", src, re.S).group(1)
    wind15 = {int(float(r)): float(m) for r, m in re.findall(r"\(([\d.]+),\s*([\d.]+)\)", w15)}
    return bdc, wind, leads, wind15


def main():
    fetch()
    scale, y0, stem_top, stem_bottom, hashes, rows, leads, numerals, bar_x, brackets = decode_mils_page()
    bdc, wind, rs_leads, wind15 = rust_constants()
    bad = []

    def check(name, got, want, tol=TOL, ref="reticle.rs"):
        ok = abs(got - want) <= tol
        print(f"  {'ok ' if ok else 'BAD'} {name:<28} extracted {got:7.3f}  {ref:<10} {want:7.3f}")
        if not ok:
            bad.append(name)

    print("\nBDC marks (MIL below crosshair) and 18-inch width check:")
    ranges = [350, 400, 450, 500, 550, 600, 650, 700, 750, 800]
    for r, (m, half) in zip(ranges, hashes):
        check(f"{r} yd hash", m, bdc[r])
        if r % 100 == 0 and r > 400:
            print(f"      half-width {half:.3f} MIL vs 9 in at {r} yd = {9 / (r * 0.036):.3f}")
    check("300 yd (stem tip)", stem_top, bdc[300])

    leaders = decode_bdc_page(stem_top, stem_bottom)
    hash_mil = dict(zip(ranges, (h[0] for h in hashes)))
    bias = sum(hash_mil[r] - m for r, m in zip([400, 500, 600, 700, 800], leaders[3:])) / 5
    print(f"\nLeader lines from the BDC figure (schematic). Leaders sit {bias:.3f} MIL above the hashes they")
    print("point at on average (400-800), so that bias is added back for 100 and 200:")
    for r, m in zip([100, 200, 300, 400, 500, 600, 700, 800], leaders):
        print(f"      {r} yd leader {m:6.3f}  +bias {m + bias:6.3f}")
    check("100 yd (chevron tip)", leaders[0] + bias, bdc[100], TOL_LEADER)
    check("200 yd (leader only)", leaders[1] + bias, bdc[200], TOL_LEADER)

    print("\nWind dots (MIL from stem), rows 500-800; 400's 5 mph hold is the hash end:")
    hash_by_range = dict(zip(ranges, hashes))
    for r in [400, 500, 600, 700, 800]:
        row = sorted(rows.get(round(hash_by_range[r][0], 1), []))
        offs = [sum(g) / len(g) for g in group(row)]
        if r == 400:
            check("400 yd 5 mph (hash end)", hash_by_range[400][1], wind[400][0])
            check("400 yd 10 mph", offs[-1], wind[400][1])
        else:
            check(f"{r} yd 5 mph", offs[0], wind[r][0])
            check(f"{r} yd 10 mph", offs[-1], wind[r][1])

    print("\nScale vs the manual's own dimensions:")
    for got, want in zip(bar_x, [10, 9, 8, 7, 6]):
        check(f"ranging bar at {want} MIL", got, want, 0.01, "manual")
    check("'6 MILS' bracket", brackets[0], 6.0, 0.03, "manual")
    check("'1 MIL' bracket", brackets[1], 1.0, 0.03, "manual")

    print("\n15 mph holds (row numerals), and 3x the 5 mph hold on the same row:")
    for r in [400, 600, 800]:
        got = numerals[round(hash_by_range[r][0], 1)]
        check(f"{r} yd 15 mph (numeral)", got, wind15[r])
        print(f"      3 x 5 mph hold = {3 * wind[r][0]:.3f}, 1.5 x 10 mph hold = {1.5 * wind[r][1]:.3f}")

    print("\nLead dots on the crosshair:")
    for got, want, name in zip(leads, rs_leads, ["3 mph", "6 mph", "9 mph"]):
        check(name, got, want)

    print(f"\n{'ALL MATCH' if not bad else 'MISMATCH: ' + ', '.join(bad)}")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
