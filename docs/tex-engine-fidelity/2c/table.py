# Interleaved log -> per-doc table: median of the two 5-run medians per build.
import sys, statistics as st, collections
r = collections.defaultdict(lambda: collections.defaultdict(lambda: ([], [])))
for line in open(sys.argv[1]):
    p = line.split()
    if len(p) < 4 or not p[2][0].isdigit(): continue
    v = "base" if p[0].startswith("base") else "new"
    r[p[1]][v][0].append(float(p[2])); r[p[1]][v][1].append(float(p[3]))
print("| Document | Cold before → after s | Δ | Warm before → after s | Δ |\n|---|---|---|---|---|")
cb, cn, wb, wn, worst = [], [], [], [], []
for d in sorted(r):
    b, n = r[d]["base"], r[d]["new"]
    c0, c1, w0, w1 = st.median(b[0]), st.median(n[0]), st.median(b[1]), st.median(n[1])
    cb.append(c0); cn.append(c1); wb.append(w0); wn.append(w1)
    worst.append((max(c1/c0, w1/w0), d))
    print(f"| {d} | {c0:.3f} → {c1:.3f} | {100*(c1/c0-1):+.0f}% | {w0:.3f} → {w1:.3f} | {100*(w1/w0-1):+.0f}% |")
print(f"| **median** | {st.median(cb):.3f} → {st.median(cn):.3f} | {100*(st.median(cn)/st.median(cb)-1):+.0f}% | {st.median(wb):.3f} → {st.median(wn):.3f} | {100*(st.median(wn)/st.median(wb)-1):+.0f}% |")
print("worst ratio:", max(worst))
