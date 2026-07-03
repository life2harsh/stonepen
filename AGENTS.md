# Stonepen Agent Context

Read `SKILLS.md` first.

Hard rules:
- Work only inside this repo.
- Build Stonepen as a standalone Rust/WASM vector ink engine.
- No React, no TypeScript app logic, no egui, no Tauri, no Dioxus.
- JavaScript is bootstrap only.
- Rust owns document model, input state, rendering, indexing, undo/redo, serialization, and export.
- Use compact naming: `press`, `min_press`, `max_press`, `base_w`, `pts`, `raw_pts`, `pos`, `screen_pos`, `world_pos`, `xform`, `bbox`, `idx`, `sel`, `doc`.
- Use typed IDs.
- Use world-coordinate vector strokes.
- Store both `raw_pts` and processed `pts`.
- Preserve draw order with `Vec`.
- Use an R-tree over stroke bounding boxes from the start.
- Runtime indexes/caches are not serialized.
- Use reversible transaction-based undo/redo.
- Draw in document order, not R-tree result order.
- Run `cargo fmt` and `cargo test -p stonepen-core`.
- Commit each working checkpoint.
