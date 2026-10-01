"""KL divergence between M193 and 77gr OTM impact distributions on the G2 BDC.

    cargo run --release --bin sensitivity > out/sensitivity.csv
    uv run --with numpy --with scipy --with pandas scripts/kl.py out

Each shot draws a muzzle velocity, an uncalled crosswind, and an angular dispersion, then
lands where the solver says (interpolated on the velocity grid from `sensitivity`). The
shooter holds the BDC mark for the range with a 50 yd zero, the same for both loads.
"""
import json
import sys

import numpy as np
import pandas as pd
from scipy.spatial import cKDTree

d = sys.argv[1] if len(sys.argv) > 1 else "out"
grid = pd.read_csv(f"{d}/sensitivity.csv")
rng = np.random.default_rng(7)

N = 200_000  # samples per load per range (moments, Monte Carlo)
N_KNN = 20_000  # samples for the nonparametric estimator
WIND_SD_MPH = 3.0  # error in the shooter's wind call, same for both loads
LOADS = {
    "M193 55gr": dict(short="M193", mv_sd=35.0, disp_moa=0.75, mv=3032),
    "Hornady 77gr OTM": dict(short="77gr", mv_sd=15.0, disp_moa=0.35, mv=2600),
}
MOA_IN_PER_YD = 1.0471975 / 100  # inches per MOA per yard
LN2 = np.log(2)


def sample(load, r, n):
    g = grid[(grid.load == load) & (grid.range_yd == r)].sort_values("mv")
    p = LOADS[load]
    mv = rng.normal(p["mv"], p["mv_sd"], n)
    wind = rng.normal(0, WIND_SD_MPH, n)
    disp = rng.normal(0, p["disp_moa"] * MOA_IN_PER_YD * r, (n, 2))
    f = lambda col: np.interp(mv, g.mv, g[col])
    hold_in = r * 36 * np.tan(g.hold_mil.iloc[0] / 1000)
    x = f("drift_in_per_mph") * wind + f("spin_in") + disp[:, 0]
    y = f("elev_in") + hold_in + f("jump_in_per_mph") * wind + disp[:, 1]
    return np.column_stack([x, y])


def exact_logpdf(load, r, X, nodes=48):
    """Exact log density of the sampling model at X (nats).

    Given the muzzle velocity, a shot is Gaussian (wind and dispersion are Gaussian and
    act linearly), so the impact density is a Gaussian mixture over velocity; integrate
    it with Gauss-Hermite quadrature.
    """
    g = grid[(grid.load == load) & (grid.range_yd == r)].sort_values("mv")
    p = LOADS[load]
    z, w = np.polynomial.hermite_e.hermegauss(nodes)
    w = w / w.sum()
    mv = p["mv"] + p["mv_sd"] * z
    f = lambda col: np.interp(mv, g.mv, g[col])
    hold_in = r * 36 * np.tan(g.hold_mil.iloc[0] / 1000)
    dr, jp = f("drift_in_per_mph"), f("jump_in_per_mph")
    s2 = (p["disp_moa"] * MOA_IN_PER_YD * r) ** 2
    terms = []
    for i in range(nodes):
        m = np.array([f("spin_in")[i], f("elev_in")[i] + hold_in])
        v = np.array([dr[i], jp[i]])
        S = WIND_SD_MPH**2 * np.outer(v, v) + s2 * np.eye(2)
        terms.append(np.log(w[i]) + log_gauss(X, m, S))
    T = np.vstack(terms)
    mx = T.max(0)
    return mx + np.log(np.exp(T - mx).sum(0))


def gauss_kl(mp, Sp, mq, Sq):
    """KL(N(mp,Sp) || N(mq,Sq)) in bits, split into mean-shift and shape parts."""
    Sq_inv = np.linalg.inv(Sq)
    dm = mq - mp
    mean_term = 0.5 * dm @ Sq_inv @ dm
    shape_term = 0.5 * (np.trace(Sq_inv @ Sp) - 2 + np.log(np.linalg.det(Sq) / np.linalg.det(Sp)))
    return (mean_term + shape_term) / LN2, mean_term / LN2, shape_term / LN2


def knn_kl(P, Q, k=5):
    """Wang-Kulkarni-Verdu (2009) k-NN estimator of KL(P||Q), bits."""
    n, m, dim = len(P), len(Q), P.shape[1]
    rho = cKDTree(P).query(P, k + 1)[0][:, k]  # skip self
    nu = cKDTree(Q).query(P, k)[0][:, k - 1]
    return (dim * np.mean(np.log(nu / rho)) + np.log(m / (n - 1))) / LN2


def hist_kl(P, Q, bin_in, smooth):
    lo = np.minimum(P.min(0), Q.min(0))
    hi = np.maximum(P.max(0), Q.max(0))
    bins = [np.arange(lo[i], hi[i] + bin_in, bin_in) for i in range(2)]
    hp = np.histogram2d(*P.T, bins=bins)[0].ravel() + smooth
    hq = np.histogram2d(*Q.T, bins=bins)[0].ravel() + smooth
    p, q = hp / hp.sum(), hq / hq.sum()
    m = p > 0
    if np.any(q[m] == 0):
        return float("inf"), int(np.sum(m & (q == 0)))
    return float(np.sum(p[m] * np.log2(p[m] / q[m]))), 0


def log_gauss(X, m, S):
    Si = np.linalg.inv(S)
    D = X - m
    return -0.5 * np.einsum("ij,jk,ik->i", D, Si, D) - 0.5 * np.log(np.linalg.det(2 * np.pi * S))


out = {"assumptions": dict(wind_sd_mph=WIND_SD_MPH, loads=LOADS, n=N, n_knn=N_KNN), "ranges": {}}
names = list(LOADS)
for r in sorted(grid.range_yd.unique()):
    S = {k: sample(k, r, N) for k in names}
    fit = {k: (S[k].mean(0), np.cov(S[k].T)) for k in names}
    row = {"loads": {}}
    for k in names:
        m, C = fit[k]
        row["loads"][LOADS[k]["short"]] = dict(
            mean=m.round(3).tolist(),
            cov=C.round(4).tolist(),
            pts=S[k][:1200].round(2).tolist(),
        )
    for a, b in [(names[0], names[1]), (names[1], names[0])]:
        tag = f"{LOADS[a]['short']}||{LOADS[b]['short']}"
        total, mean_t, shape_t = gauss_kl(*fit[a], *fit[b])
        # Ground truth for this model: Monte Carlo of E_P[log p - log q] with exact densities.
        Xa = S[a][:50_000]
        # Per-shot evidence (bits) that the shot came from `a` rather than `b`.
        llr = (exact_logpdf(a, r, Xa) - exact_logpdf(b, r, Xa)) / LN2
        exact = float(np.mean(llr))
        knn = knn_kl(S[a][:N_KNN], S[b][:N_KNN])
        hist_naive, empty = hist_kl(S[a][:N_KNN], S[b][:N_KNN], 1.0, 0.0)
        hist_smooth, _ = hist_kl(S[a][:N_KNN], S[b][:N_KNN], 1.0, 0.5)
        # Identify the load from n shots by total log-likelihood (truth = a, equal priors).
        ident = []
        for n in range(1, 11):
            tot = llr[: (len(llr) // n) * n].reshape(-1, n).sum(1)
            ident.append(round(float(np.mean(tot > 0)), 4))
        row[tag] = dict(
            exact=round(exact, 4),
            gauss=round(total, 4),
            mean_term=round(mean_t, 4),
            shape_term=round(shape_t, 4),
            knn=round(knn, 4),
            hist_naive=hist_naive if np.isfinite(hist_naive) else None,
            hist_naive_empty_bins=empty,
            hist_smooth=round(hist_smooth, 4),
            llr_mean=round(float(llr.mean()), 4),
            llr_sd=round(float(llr.std()), 4),
            llr_samples=llr[:3000].round(3).tolist(),
            llr_walks=[np.cumsum(llr[i * 25:(i + 1) * 25]).round(3).tolist() for i in range(24)],
            identify=ident,
        )
    out["ranges"][int(r)] = row
    a, b = "M193||77gr", "77gr||M193"
    print(
        f"{r:4.0f} yd  KL(M193||77) exact {row[a]['exact']:7.3f} gauss {row[a]['gauss']:7.3f} knn {row[a]['knn']:7.3f} hist~ {row[a]['hist_smooth']:7.3f}"
        f" | KL(77||M193) exact {row[b]['exact']:7.3f} gauss {row[b]['gauss']:7.3f} knn {row[b]['knn']:7.3f} hist~ {row[b]['hist_smooth']:7.3f}"
        f" | naive-hist empty bins {row[a]['hist_naive_empty_bins']}/{row[b]['hist_naive_empty_bins']}"
    )

with open(f"{d}/kl.json", "w") as f:
    json.dump(out, f, separators=(",", ":"))
print(f"wrote {d}/kl.json")
