"""Precompute the ancestor closure core.dll uses as the iterator's descend gate.

Mirrors DB_Noun::recurseEleTypes(ATT_OWNER, results): a transitive closure over
the owner relation, seeded from each requested type, excluding the seeds unless
they are reachable as ancestors in their own right.
"""

import json
from collections import deque

GRAPH = "noun_graph_full.json"
FLAGS = "noun_flags.json"
OUT = "noun_closure.json"


def load():
    """Build both directions from owners *and* the reverse of members.

    A handful of types only ever appear as a member name and carry no descriptor
    of their own; without the reverse edges their owners drop out of the closure.
    """
    g = json.load(open(GRAPH, encoding="utf-8"))
    flags = {r["noun"]: r for r in json.load(open(FLAGS, encoding="utf-8"))}

    owners, members = {}, {}
    for child, v in g.items():
        owners.setdefault(child, set()).update(v["owners"])
        for owner in v["owners"]:
            members.setdefault(owner, set()).add(child)
        for member in v["members"]:
            members.setdefault(child, set()).add(member)
            owners.setdefault(member, set()).add(child)

    owners = {k: sorted(v) for k, v in owners.items()}
    members = {k: sorted(v) for k, v in members.items()}
    return g, flags, owners, members


def closure(seeds, adj):
    """Transitive closure, seeds themselves only included if re-reached."""
    seen, queue = set(), deque()
    for s in seeds:
        for nxt in adj.get(s, ()):
            if nxt not in seen:
                seen.add(nxt)
                queue.append(nxt)
    while queue:
        cur = queue.popleft()
        for nxt in adj.get(cur, ()):
            if nxt not in seen:
                seen.add(nxt)
                queue.append(nxt)
    return seen


def main():
    g, flags, owners, members = load()
    types = sorted(g)

    ancestors = {n: sorted(closure([n], owners)) for n in types}
    descendants = {n: sorted(closure([n], members)) for n in types}

    print(f"types={len(types)}")
    sizes = sorted((len(v), k) for k, v in ancestors.items())
    print(f"ancestor-set size: min={sizes[0]} median={sizes[len(sizes) // 2]} max={sizes[-1]}")

    sets = {}
    for flag in ("PRMF", "GORP", "XTRF", "POPF"):
        seeds = [n for n in types if flags.get(n, {}).get(flag) == 1]
        gate = closure(seeds, owners)
        sets[flag] = {"seeds": sorted(seeds), "descend_gate": sorted(gate)}
        pct = 100.0 * len(gate) / len(types)
        print(f"  {flag:6s} seeds={len(seeds):4d}  descend gate={len(gate):4d} "
              f"({pct:.1f}% of types)  -> prunes {len(types) - len(gate)} types")

    print("\n-- example: gate for a single target type --")
    for t in ("ELBO", "BOX", "SCTN", "NOZZ"):
        if t in ancestors:
            a = ancestors[t]
            print(f"  {t:5s} ancestors({len(a)}): {a}")

    print("\n-- what a PRMF walk from these roots would descend into --")
    gate = set(sets["PRMF"]["descend_gate"])
    for root in ("SITE", "ZONE", "EQUI", "PIPE", "STRU"):
        if root not in descendants:
            continue
        below = set(descendants[root])
        print(f"  {root:5s} reachable below={len(below):4d}  "
              f"of which worth descending={len(below & gate):4d}  "
              f"skipped={len(below - gate):4d}")

    json.dump(
        {
            "ancestors": ancestors,
            "descendants": descendants,
            "flag_gates": sets,
        },
        open(OUT, "w", encoding="utf-8"),
        indent=1,
        ensure_ascii=False,
    )
    print(f"\nwrote {OUT}")


if __name__ == "__main__":
    main()
