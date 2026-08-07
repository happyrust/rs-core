"""Emit the petgraph-compatible noun_graph.json plus a depth report for rs-core."""

import json
from collections import deque

BASE27 = 0x81BF1


def rehash(name):
    val = 0
    for ch in reversed(name.upper()):
        val = val * 27 + (0 if ch == " " else ord(ch) - 64)
    return val + BASE27


graph = json.load(open("noun_graph_full.json", encoding="utf-8"))
flags = {r["noun"]: r for r in json.load(open("noun_flags.json", encoding="utf-8"))}

# Every name that shows up anywhere, so member references never dangle.
names = set(graph)
for v in graph.values():
    names.update(v["owners"])
    names.update(v["members"])
names = sorted(names)
idx = {n: i for i, n in enumerate(names)}

# Edge direction matches the existing convention: child -> owner.
edges = set()
for child, v in graph.items():
    for owner in v["owners"]:
        edges.add((idx[child], idx[owner]))
    for member in v["members"]:
        edges.add((idx[member], idx[child]))
edges = sorted(edges)

petgraph = {
    "nodes": [rehash(n) for n in names],
    "node_holes": [],
    "edge_property": "directed",
    "edges": [[a, b, 0] for a, b in edges],
}
json.dump(petgraph, open("noun_graph.json", "w", encoding="utf-8"))
print(f"nodes={len(names)} edges={len(edges)}")

# Depth characteristics: how far apart are types along the owner chain?
adj = {}
for a, b in edges:
    adj.setdefault(a, []).append(b)


def bfs(src):
    dist = {src: 0}
    q = deque([src])
    while q:
        cur = q.popleft()
        for nxt in adj.get(cur, ()):
            if nxt not in dist:
                dist[nxt] = dist[cur] + 1
                q.append(nxt)
    return dist


worst = 0
worst_pair = None
for n in ("ELBO", "BOX", "VERT", "PTAX", "SNOD", "FITT", "NBOX", "CMFI"):
    if n not in idx:
        continue
    d = bfs(idx[n])
    for tgt, dist in d.items():
        if dist > worst:
            worst, worst_pair = dist, (n, names[tgt])
print(f"deepest owner chain from sampled leaves: {worst} hops via {worst_pair}")

allmax = 0
for n in names:
    d = bfs(idx[n])
    if d:
        m = max(d.values())
        allmax = max(allmax, m)
print(f"max shortest-path depth over all types: {allmax}")

cyc = [n for n in graph if n in graph[n]["owners"]]
print(f"self-owning types (cycles of length 1): {len(cyc)} e.g. {cyc[:10]}")
