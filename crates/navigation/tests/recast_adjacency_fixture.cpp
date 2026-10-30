// Exercise the bundled Recast implementation itself, including its private adjacency builder.
#include "../../../vendor/recastnavigation-sys/recastnavigation/Recast/Source/RecastMesh.cpp"
#include <cstdio>
#include <map>
#include <utility>
#include <vector>

int main() {
    const int side = 180, nvp = 6, npolys = side * side;
    const int base = (side + 1) * (side + 1), nverts = base + (side + 1) * side;
    std::vector<unsigned short> polys(npolys * nvp * 2, RC_MESH_NULL_IDX);
    auto vertex = [side](int x, int z) { return z * (side + 1) + x; };
    auto midpoint = [base, side](int x, int z) { return base + x * side + z; };
    std::map<std::pair<int, int>, std::pair<int, int>> edges;
    int ascending = 0, shared = 0;
    for (int z = 0; z < side; ++z) {
        for (int x = 0; x < side; ++x) {
            const int poly = z * side + x;
            int points[6] = {vertex(x, z), vertex(x + 1, z), midpoint(x + 1, z),
                             vertex(x + 1, z + 1), vertex(x, z + 1), midpoint(x, z)};
            for (int edge = 0; edge < nvp; ++edge) {
                int a = points[edge], b = points[(edge + 1) % nvp];
                polys[poly * nvp * 2 + edge] = static_cast<unsigned short>(a);
                ascending += a < b;
                edges[{a, b}] = {poly, edge};
            }
        }
    }
    if (ascending <= 65535 || !buildMeshAdjacency(polys.data(), npolys, nverts, nvp)) return 1;
    for (const auto& [edge, location] : edges) {
        auto reverse = edges.find({edge.second, edge.first});
        const auto actual = polys[location.first * nvp * 2 + nvp + location.second];
        if (reverse == edges.end()) {
            if (actual != RC_MESH_NULL_IDX) return 2;
        } else {
            ++shared;
            if (actual != reverse->second.first) {
                std::fprintf(stderr, "missing adjacency: polygon %d edge %d, expected %d got %u\n",
                             location.first, location.second, reverse->second.first, actual);
                return 3;
            }
        }
    }
    std::printf("%d ascending edges, %d linked shared directed edges\n", ascending, shared);
    return shared > 0 ? 0 : 4;
}
