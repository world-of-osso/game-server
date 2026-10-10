# recastnavigation-sys 1.0.3

Source: crates.io recastnavigation-sys 1.0.3, MIT bindings plus bundled Zlib RecastNavigation 1.6.0 (licenses retained). Geometry/nav algorithms are unmodified.

Local build compatibility changes:
- bindgen 0.69 → 0.72.1: with host libclang, 0.69 emitted dtNavMeshQuery as opaque and omitted all its methods. Updating bindgen generates query bindings; no hand-maintained C++ ABI declarations.
- CMake minimum 3.1 → 3.5: CMake4 removed older policy compatibility. Fix declaration rather than environment policy override.
- RustTarget::Nightly → RustTarget::nightly(), and scope inline Detour imports to tile-cache, which actually needs them. Fix warnings from updated bindings.

Always build bundled source (root .cargo/config.toml). Remove patch when an upstream release contains these compatibility fixes and real tiled Stockade/Stormwind fixtures pass. No system packages installed.
