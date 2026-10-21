#!/usr/bin/env python3
"""Freeze FOLLOWER_ROUTE_BOSS_PATH_CORRIDOR from native graph/radius inputs."""
import argparse
import heapq
import json
import math
from pathlib import Path

POLICY = "FOLLOWER_ROUTE_BOSS_PATH_CORRIDOR"


def point(pack):
    return [pack["position"][0], pack["height"], pack["position"][1]]


def squared_distance(a, b):
    return sum((left - right) ** 2 for left, right in zip(a, b))


def build_adjacency(graph):
    adjacency = {pack["id"]: {} for pack in graph["packs"]}
    for directed, edges in [(False, graph["edges"]), (True, graph["teleports"])]:
        for edge in edges:
            a, b, yards = edge["from"], edge["to"], edge["yards"]
            if a not in adjacency or b not in adjacency or not math.isfinite(yards) or yards < 0:
                raise ValueError(f"invalid frozen edge {edge}")
            adjacency[a][b] = yards
            if not directed:
                adjacency[b][a] = yards
    return adjacency


def shortest_path(adjacency, start, goal):
    distances, previous = {start: 0.0}, {}
    pending = [(0.0, start)]
    while pending:
        distance, node = heapq.heappop(pending)
        if distance != distances[node]:
            continue
        if node == goal:
            path = [goal]
            while path[-1] != start:
                path.append(previous[path[-1]])
            return list(reversed(path))
        for other, yards in sorted(adjacency[node].items()):
            candidate = distance + yards
            if other not in distances or candidate < distances[other]:
                distances[other] = candidate
                previous[other] = node
                heapq.heappush(pending, (candidate, other))
    raise ValueError(f"no frozen path from pack{start} to pack{goal}")


def sphere_intersection(center, radius, start, end):
    """Return first contact fraction and closest corridor anchor, or no intersection."""
    direction = [b - a for a, b in zip(start, end)]
    relative = [a - c for a, c in zip(start, center)]
    length_squared = sum(value * value for value in direction)
    if length_squared == 0:
        return (0.0, list(start)) if squared_distance(center, start) <= radius * radius else None
    linear = 2 * sum(a * b for a, b in zip(relative, direction))
    constant = sum(value * value for value in relative) - radius * radius
    discriminant = linear * linear - 4 * length_squared * constant
    if discriminant < 0:
        return None
    root = math.sqrt(discriminant)
    enter = (-linear - root) / (2 * length_squared)
    leave = (-linear + root) / (2 * length_squared)
    if leave < 0 or enter > 1:
        return None
    closest = min(1.0, max(0.0, -linear / (2 * length_squared)))
    anchor = [a + closest * delta for a, delta in zip(start, direction)]
    return max(0.0, enter), anchor


def walk_segments(graph, packs, adjacency, nodes, origin):
    teleports = {(edge["from"], edge["to"]) for edge in graph["teleports"]}
    current, segments = list(origin), []
    for a, b in zip(nodes, nodes[1:]):
        destination = point(packs[b])
        if (a, b) in teleports:
            source = point(packs[a])
            if current != source:
                segments.append((current, source, math.sqrt(squared_distance(current, source))))
        else:
            segments.append((current, destination, adjacency[a][b]))
        current = destination
    destination = point(packs[nodes[-1]])
    if current != destination:
        segments.append((current, destination, math.sqrt(squared_distance(current, destination))))
    return segments


def validate_sources(graph, profile):
    if graph["map_id"] != profile["map_id"]:
        raise ValueError("native radius profile belongs to another map")
    expected = {spawn["guid"] for pack in graph["packs"] for spawn in pack["spawns"]}
    sources = {source["guid"]: source for source in profile["sources"]}
    if expected != set(sources) or len(sources) != len(profile["sources"]):
        raise ValueError("recapture native corridor inputs after source population changes")
    for source in sources.values():
        radius = source["radius"]
        if source["eligible"] and (radius is None or not math.isfinite(radius) or radius <= 0):
            raise ValueError(f"missing native acquisition radius for source{source['guid']}")
    return sources


def packs_for_entry(packs, entry):
    return sorted(pack["id"] for pack in packs.values()
                  if any(spawn["entry"] == entry for spawn in pack["spawns"]))


def boss_packs(packs, requirements):
    bosses = []
    for entry in requirements["boss_order"]:
        found = packs_for_entry(packs, entry)
        if len(found) != 1:
            raise ValueError(f"boss{entry} requires one unambiguous source pack: {found}")
        bosses.append(found[0])
    return bosses


def gate_packs(packs, requirements, boss_entry):
    gates = set()
    for entry in requirements["gate_entries"].get(str(boss_entry), []):
        found = packs_for_entry(packs, entry)
        if not found:
            raise ValueError(f"boss{boss_entry} gate entry{entry} lacks a source pack")
        gates.update(found)
    return sorted(gates)


def corridor_pulls(packs, sources, segments, excluded):
    candidates, elapsed = {}, 0.0
    for start, end, yards in segments:
        for pack in packs.values():
            if pack["id"] in excluded:
                continue
            for spawn in pack["spawns"]:
                source = sources[spawn["guid"]]
                if not source["eligible"]:
                    continue
                contact = sphere_intersection(source["position"], source["radius"], start, end)
                if contact is None:
                    continue
                fraction, anchor = contact
                encounter = elapsed + fraction * yards
                previous = candidates.get(pack["id"])
                if previous is None or encounter < previous[0]:
                    candidates[pack["id"]] = (encounter, anchor)
        elapsed += yards
    return [{"pack": key, "anchor": value[1]}
            for key, value in sorted(candidates.items(), key=lambda item: (item[1][0], item[0]))]


def plan_route(graph, profile, requirements):
    sources = validate_sources(graph, profile)
    packs = {pack["id"]: pack for pack in graph["packs"]}
    adjacency = build_adjacency(graph)
    bosses = boss_packs(packs, requirements)
    origin = [graph["entrance"][0], graph["entrance_height"], graph["entrance"][1]]
    visited, route, boss_path = set(), [], []
    for entry, boss in zip(requirements["boss_order"], bosses):
        for goal in gate_packs(packs, requirements, entry) + [boss]:
            if goal in visited:
                continue
            nearest = min(packs, key=lambda key: (squared_distance(point(packs[key]), origin), key))
            nodes = shortest_path(adjacency, nearest, goal)
            segments = walk_segments(graph, packs, adjacency, nodes, origin)
            pulls = corridor_pulls(packs, sources, segments, visited | set(bosses) | {goal})
            for pull in pulls:
                route.append(pull)
                visited.add(pull["pack"])
            route.append({"pack": goal, "anchor": point(packs[goal])})
            visited.add(goal)
            boss_path.extend(nodes[1:] if boss_path and boss_path[-1] == nodes[0] else nodes)
            origin = point(packs[goal])
    return {"route": route, "boss_path": boss_path, "route_policy": POLICY}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--graph", type=Path, required=True)
    parser.add_argument("--inputs", type=Path, required=True)
    parser.add_argument("--requirements", type=Path, required=True)
    args = parser.parse_args()
    graph = json.loads(args.graph.read_text())
    profile = json.loads(args.inputs.read_text())
    requirements = json.loads(args.requirements.read_text())[str(graph["map_id"])]
    result = plan_route(graph, profile, requirements)
    graph.update(result)
    args.graph.write_text(json.dumps(graph, indent=2) + "\n")
    print(f"{POLICY} map={graph['map_id']} pulls={len(result['route'])} "
          f"path_nodes={len(result['boss_path'])}")


if __name__ == "__main__":
    main()
