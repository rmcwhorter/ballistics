"""Chart the BDC calibration from `bdc --csv out`.  uv run --with matplotlib --with pandas scripts/plot.py out"""
import sys
import matplotlib.pyplot as plt
import pandas as pd

d = sys.argv[1] if len(sys.argv) > 1 else "out"
miss = pd.read_csv(f"{d}/miss.csv")
wind = pd.read_csv(f"{d}/wind.csv")

LOADS = ["M193 55gr FMJBT", "SS109/M855 62gr", "Hornady 77gr OTM"]
SHORT = {"M193 55gr FMJBT": "M193 @ 3032", "SS109/M855 62gr": "SS109 @ 2808", "Hornady 77gr OTM": "77gr OTM @ 2650"}
COLOR = dict(zip(LOADS, ["#2a78d6", "#eb6834", "#1baf7a"]))
INK, MUTED, GRID, SURF = "#0b0b0b", "#52514e", "#e4e3df", "#fcfcfb"
FLOOR, TOL = -50, 6

plt.rcParams.update({"font.family": "Helvetica", "font.size": 11, "axes.edgecolor": GRID,
                     "axes.labelcolor": MUTED, "xtick.color": MUTED, "ytick.color": MUTED})
fig, axes = plt.subplots(2, 2, figsize=(15, 10.5), facecolor=SURF)
fig.subplots_adjust(left=0.06, right=0.97, top=0.84, bottom=0.11, hspace=0.38, wspace=0.16)

def style(ax):
    ax.set_facecolor(SURF)
    ax.grid(axis="y", color=GRID, lw=0.8)
    ax.spines[["top", "right"]].set_visible(False)
    ax.tick_params(length=0)

panels = [("50 yd zero", "50-yard zero  (what the reticle is designed for)"),
          ("100 yd zero", "100-yard zero"),
          ("best zero", "Best zero per load  (every mark within ±6\" for longest)")]
for ax, (z, title) in zip([axes[0, 0], axes[0, 1], axes[1, 0]], panels):
    style(ax)
    ax.axhspan(-TOL, TOL, color="#eeede9", zorder=0)
    ax.axhline(0, color=MUTED, lw=1, zorder=1)
    ax.text(805, TOL - 0.6, f"±{TOL}\" hit zone", ha="right", va="top", color=MUTED, fontsize=9.5)
    for k, load in enumerate(LOADS):
        s = miss[(miss.load == load) & (miss.zero == z) & (miss.range_yd != 200)]
        y = s.miss_in.clip(lower=FLOOR)
        ax.plot(s.range_yd, y, color=COLOR[load], lw=2, zorder=3, solid_capstyle="round")
        ax.scatter(s.range_yd, y, s=36, color=COLOR[load], edgecolor=SURF, linewidth=1.5, zorder=4)
        for r, v in zip(s.range_yd, s.miss_in):
            if v < FLOOR:
                ax.annotate(f"{v:.0f}\"", (r, FLOOR), xytext=(0, -12 - 11 * k), textcoords="offset points",
                            ha="center", fontsize=8.5, color=COLOR[load], fontweight="bold")
        off = s.zero_offset_in_at_100.iloc[0]
        if z == "best zero":
            last = s[s.miss_in >= FLOOR].iloc[-1]
    ax.set_xlim(80, 820)
    ax.set_ylim(FLOOR - 16, 16)
    ax.set_xticks(range(100, 801, 100))
    ax.set_title(title, loc="left", color=INK, fontsize=13, fontweight="bold", pad=10)
    ax.set_ylabel("impact vs. aim, inches  (+ high)")
    ax.set_xlabel("range, yards  (holding the BDC mark for that range)")

# Zero offsets for the best-zero panel, as text in the panel.
ax = axes[1, 0]
lines = []
for load in LOADS:
    off = miss[(miss.load == load) & (miss.zero == "best zero")].zero_offset_in_at_100.iloc[0]
    lines.append(f"{SHORT[load]}: {off:+.2f}\" at 100 yd")
ax.text(95, -30, "\n".join(lines), fontsize=10, color=INK, va="top",
        bbox=dict(boxstyle="round,pad=0.5", fc=SURF, ec=GRID))

# Wind panel: what the "10 mph" dot actually equals.
ax = axes[1, 1]
style(ax)
ax.axhline(10, color=MUTED, lw=1, ls=(0, (4, 3)))
ax.text(385, 9.9, "reticle says 10 mph", ha="left", va="top", color=MUTED, fontsize=9.5)
for load in LOADS:
    s = wind[wind.load == load]
    ax.plot(s.range_yd, s.dot10_mph, color=COLOR[load], lw=2)
    ax.scatter(s.range_yd, s.dot10_mph, s=36, color=COLOR[load], edgecolor=SURF, linewidth=1.5, zorder=4)
    ax.annotate(f"{s.dot10_mph.iloc[-1]:.1f} mph", (800, s.dot10_mph.iloc[-1]), xytext=(10, 6 if load == LOADS[2] else 0),
                textcoords="offset points", va="center", fontsize=10, color=INK)
ax.set_xlim(380, 860)
ax.set_ylim(5, 11.5)
ax.set_xticks(range(400, 801, 100))
ax.set_title("Wind: the \"10 mph\" dot actually equals…", loc="left", color=INK, fontsize=13, fontweight="bold", pad=10)
ax.set_ylabel("full-value crosswind, mph")
ax.set_xlabel("range, yards  (5 mph dot = half of these)")

fig.text(0.06, 0.958, "PLxC 1-8x24 FFP RDB  ·  ACSS Raptor 5.56/.308 Yard G2 BDC  -  how three loads calibrate",
         fontsize=19, fontweight="bold", color=INK)
fig.text(0.06, 0.93, "Point-mass G7 solver (cross-checked vs. py-ballisticcalc), reticle geometry from Primary Arms' vector manual art.",
         fontsize=11.5, color=MUTED)
handles = [plt.Line2D([], [], color=COLOR[l], lw=2, marker="o", ms=6, mec=SURF) for l in LOADS]
fig.legend(handles, [SHORT[l] + " fps" for l in LOADS], loc="upper left", bbox_to_anchor=(0.055, 0.92),
           ncol=3, frameon=False, fontsize=11.5, handlelength=2.2)
fig.text(0.06, 0.03,
         "Assumptions: ICAO sea level (59 °F, 29.92 inHg), 2.75\" sight height, 1:7 twist. G7 BCs: M193 0.120, M855 0.151 (Litz), Hornady 77gr OTM 0.190 (Hornady).\n"
         "Misses below -50\" are pinned to the floor with their value. 200 yd omitted (no physical mark). Wind excludes spin drift (~0.3-0.5 mil right at 800).",
         fontsize=9.5, color=MUTED)
out = f"{d}/bdc_calibration.png"
fig.savefig(out, dpi=150, facecolor=SURF)
print(out)
