# recastnavigation-sys 1.0.3

Source: crates.io recastnavigation-sys 1.0.3, MIT bindings plus bundled Zlib RecastNavigation 1.6.0 (licenses retained).

Local changes:
- Recast polygon adjacency uses 32-bit temporary edge indices. A tile can have more than 65,535 edges even when its polygon and vertex indices fit in 16 bits; the upstream 16-bit temporary indices silently lose links. Polygon neighbor encoding is unchanged. Upstream Recast `main` still has the 16-bit indices (checked 2026-10-30). Retire this change when an upstream fix passes `recast_links_a_mesh_with_more_than_u16_max_edges`.
- bindgen 0.69 → 0.72.1: with host libclang, 0.69 emitted dtNavMeshQuery as opaque and omitted all its methods. Updating bindgen generates query bindings; no hand-maintained C++ ABI declarations.
- CMake minimum 3.1 → 3.5: CMake4 removed older policy compatibility. Fix declaration rather than environment policy override.
- RustTarget::Nightly → RustTarget::nightly(), and scope inline Detour imports to tile-cache, which actually needs them. Fix warnings from updated bindings.

Always build bundled source (root .cargo/config.toml). Remove patch when an upstream release contains these fixes and the adjacency and real tiled Stockade/Stormwind fixtures pass. No system packages installed.
