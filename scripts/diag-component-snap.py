#!/usr/bin/env python3
"""Snap a Bevy XYZ point to the nearest triangle in an exported Detour component."""
import json
import math
import sys


def sub(a, b):
    return tuple(x - y for x, y in zip(a, b))


def add(a, b):
    return tuple(x + y for x, y in zip(a, b))


def mul(a, scale):
    return tuple(x * scale for x in a)


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def closest(p, a, b, c):
    ab, ac, ap = sub(b, a), sub(c, a), sub(p, a)
    d1, d2 = dot(ab, ap), dot(ac, ap)
    if d1 <= 0 and d2 <= 0:
        return a
    bp = sub(p, b)
    d3, d4 = dot(ab, bp), dot(ac, bp)
    if d3 >= 0 and d4 <= d3:
        return b
    vc = d1 * d4 - d3 * d2
    if vc <= 0 and d1 >= 0 and d3 <= 0:
        return add(a, mul(ab, d1 / (d1 - d3)))
    cp = sub(p, c)
    d5, d6 = dot(ab, cp), dot(ac, cp)
    if d6 >= 0 and d5 <= d6:
        return c
    vb = d5 * d2 - d1 * d6
    if vb <= 0 and d2 >= 0 and d6 <= 0:
        return add(a, mul(ac, d2 / (d2 - d6)))
    va = d3 * d6 - d5 * d4
    if va <= 0 and d4 - d3 >= 0 and d5 - d6 >= 0:
        return add(b, mul(sub(c, b), (d4 - d3) / ((d4 - d3) + (d5 - d6))))
    denominator = 1 / (va + vb + vc)
    return add(a, add(mul(ab, vb * denominator), mul(ac, vc * denominator)))


def main():
    if len(sys.argv) != 5:
        raise SystemExit("usage: diag-component-snap.py COMPONENT.txt X Y Z")
    target = tuple(map(float, sys.argv[2:]))
    best = None
    with open(sys.argv[1], encoding="utf-8") as stream:
        for line in stream:
            fields = line.split()
            vertices = [tuple(map(float, fields[i:i + 3])) for i in range(2, len(fields), 3)]
            for index in range(1, len(vertices) - 1):
                point = closest(target, vertices[0], vertices[index], vertices[index + 1])
                result = (math.dist(target, point), fields[0], point)
                if best is None or result < best:
                    best = result
    if best is None:
        raise SystemExit("component has no triangles")
    print(json.dumps({"distance": best[0], "polygon": best[1], "point": best[2]}))


if __name__ == "__main__":
    main()
