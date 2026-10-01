"""Inline out/kl.json into site/template.html -> site/bits-per-shot.html."""
import json, sys
d = sys.argv[1] if len(sys.argv) > 1 else "out"
data = json.load(open(f"{d}/kl.json"))
for r in data["ranges"].values():
    for t in ("M193||77gr", "77gr||M193"):
        r[t]["llr_samples"] = [round(v, 2) for v in r[t]["llr_samples"]]
        r[t]["llr_walks"] = [[round(v, 2) for v in w] for w in r[t]["llr_walks"]]
html = open("site/template.html").read().replace("__DATA__", json.dumps(data, separators=(",", ":")))
open("site/bits-per-shot.html", "w").write(html)
print(f"site/bits-per-shot.html {len(html)/1e3:.0f} KB")
