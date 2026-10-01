"""Show that the k-NN KL estimator is biased low here even on exactly Gaussian data.

    uv run --with numpy --with scipy --with pandas scripts/knn_bias_check.py out

Draws true Gaussians with the moments fitted in out/kl.json, so the closed-form KL is the
right answer, and runs the same Wang-Kulkarni-Verdu estimator kl.py uses at growing n.
"""
import json
import sys

import numpy as np
from scipy.spatial import cKDTree

d = sys.argv[1] if len(sys.argv) > 1 else "out"
data = json.load(open(f"{d}/kl.json"))
rng = np.random.default_rng(11)
LN2 = np.log(2)


def gauss_kl(mp, Sp, mq, Sq):
    Si = np.linalg.inv(Sq)
    dm = mq - mp
    return 0.5 * (np.trace(Si @ Sp) + dm @ Si @ dm - 2 + np.log(np.linalg.det(Sq) / np.linalg.det(Sp))) / LN2


def knn_kl(P, Q, k=5):
    n, m, dim = len(P), len(Q), P.shape[1]
    rho = cKDTree(P).query(P, k + 1)[0][:, k]
    nu = cKDTree(Q).query(P, k)[0][:, k - 1]
    return (dim * np.mean(np.log(nu / rho)) + np.log(m / (n - 1))) / LN2


for r in ["100", "400", "800"]:
    L = data["ranges"][r]["loads"]
    mp, Sp = np.array(L["M193"]["mean"]), np.array(L["M193"]["cov"])
    mq, Sq = np.array(L["77gr"]["mean"]), np.array(L["77gr"]["cov"])
    est = []
    for n in [2_000, 20_000, 200_000]:
        P, Q = rng.multivariate_normal(mp, Sp, n), rng.multivariate_normal(mq, Sq, n)
        est.append(f"n={n}: {knn_kl(P, Q):.2f}")
    print(f"{r} yd  true D(M193||77gr) {gauss_kl(mp, Sp, mq, Sq):.2f} bits | k-NN " + ", ".join(est))
