# Stonepen Agent Skills

Project: Stonepen
Target: standalone Rust/WASM vector handwriting input engine.
Use this file as the working instruction set for Antigravity agents.

## Mission

Build Stonepen as a serious digital ink engine, not a toy drawing demo.

Stonepen captures handwritten ink as editable vector strokes. It does not interpret,
classify, OCR, or convert handwriting into text.

The first complete build must support:
- Rust core engine
- Rust/WASM browser app
- vector stroke capture
- pressure/tilt/twist-aware input
- smoothing and resampling
- R-tree spatial index
- lasso selection
- stroke eraser
- layers
- undo/redo transactions
- pan/zoom viewport
- save/load `.stonepen.json`
- export SVG
- export PNG
- tests for core geometry, document, spatial index, ops, and serialization

## Model guidance

Preferred Antigravity model for the main build:
- Claude Sonnet 4.6 (Thinking)

Use this for the main implementation because it balances reasoning, code generation,
and agentic iteration.

Use Claude Opus 4.6 (Thinking) only for:
- architecture review
- hard Rust ownership fixes
- diagnosing broken design decisions

Use Gemini 3.5 Flash for:
- fast compile-error loops
- small edits
- mechanical refactors
- CSS/UI polish

Avoid using fast/low reasoning models for the first full architecture pass.

## Safety rules

Work only inside the Stonepen repository.

Before large changes:

```sh
git status
```

After each coherent milestone:

```sh
git add .
git commit -m "checkpoint: <short description>"
```

Do not delete files outside the repository.
Do not run destructive commands outside the repository.
Do not run cleanup commands like `rm -rf` unless the exact path is inside the repo
and the reason is stated first.
Do not modify unrelated repositories.

## Build order

Build in phases. Do not mix all phases at once.

### Phase 1: Core crate

Create:

```txt
crates/stonepen-core/
```

Implement:
- typed IDs
- color
- points
- bounding boxes
- transforms
- brushes
- strokes
- layers
- document model
- runtime indexes
- R-tree spatial index
- geometry
- hit testing
- smoothing
- resampling
- selection
- operations
- session
- JSON export/import
- SVG export
- tests

Required command:

```sh
cargo test -p stonepen-core
```

Do not start WASM until this passes.

### Phase 2: WASM crate

Create:

```txt
crates/stonepen-wasm/
```

Implement:
- browser app wrapper
- canvas lookup
- Canvas 2D renderer
- PointerEvent adapter
- keyboard adapter
- file export helpers
- PNG export through canvas data URL

Required command:

```sh
wasm-pack build crates/stonepen-wasm --target web --out-dir ../../web/pkg
```

### Phase 3: Web shell

Create:

```txt
web/index.html
web/styles.css
web/main.js
```

Rules:
- JavaScript is bootstrap only.
- No app logic in JavaScript.
- Rust owns input state, document mutation, rendering, undo/redo, save/load, export.

### Phase 4: Browser verification

Verify:
- drawing works
- panning works
- zooming works
- eraser deletes strokes
- lasso selects strokes
- undo/redo works
- save/load restores strokes
- SVG export downloads
- PNG export downloads
- highlighter is translucent
- selected strokes are visible
- canvas resizes correctly

Take a screenshot artifact after browser verification.

## Naming style

Stonepen uses compact Rust names for engine code.

Preferred abbreviations:
- `press` for pressure
- `min_press` / `max_press` for pressure range
- `base_w` for base width
- `pts` for points
- `raw_pts` for raw points
- `pos` for position
- `screen_pos` / `world_pos` for coordinate-specific positions
- `xform` for transform
- `bbox` for bounding box
- `idx` for index
- `sel` for selection
- `doc` for document
- `tx` for transaction when local and obvious
- `op` for operation
- `dpr` for device pixel ratio

Avoid compressed no-separator names:
- no `minpress`
- no `rawpts`
- no `selectedstrokes`
- no `strokeindex`

Use snake_case with compact terms.

Good:

```rust
pub struct Brush {
    pub kind: BrushKind,
    pub color: ColorRgba,
    pub base_w: f32,
    pub min_press: f32,
    pub max_press: f32,
    pub opacity: f32,
    pub smooth: f32,
    pub streamline: f32,
}
```

Good:

```rust
pub struct InkStroke {
    pub id: StrokeId,
    pub brush: Brush,
    pub raw_pts: Vec<InkPoint>,
    pub pts: Vec<InkPoint>,
    pub local_bbox: BBox,
    pub world_bbox: BBox,
    pub xform: Xform2D,
}
```

## Architecture

Use a Rust workspace:

```txt
stonepen/
  Cargo.toml
  README.md

  crates/
    stonepen-core/
      Cargo.toml
      src/
        lib.rs
        ids.rs
        color.rs
        point.rs
        bbox.rs
        xform.rs
        brush.rs
        stroke.rs
        layer.rs
        doc.rs
        runtime.rs
        spatial.rs
        geom.rs
        resample.rs
        smooth.rs
        hit.rs
        sel.rs
        ops.rs
        session.rs
        viewport.rs
        export_json.rs
        export_svg.rs

    stonepen-wasm/
      Cargo.toml
      src/
        lib.rs
        app.rs
        pointer.rs
        canvas.rs
        render_2d.rs
        file_io.rs
        keyboard.rs

  web/
    index.html
    styles.css
    main.js
```

The core crate must not depend on browser APIs.

## Core data model

Use typed IDs:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocId(pub uuid::Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LayerId(pub uuid::Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StrokeId(pub uuid::Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BrushId(pub uuid::Uuid);
```

Use world coordinates for saved ink.
Canvas/screen coordinates are only for rendering and input conversion.

### Basic types

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BBox {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Xform2D {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub tx: f32,
    pub ty: f32,
}
```

### Brush

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ColorRgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BrushKind {
    Pen,
    Pencil,
    Highlighter,
    Marker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Brush {
    pub id: BrushId,
    pub name: String,
    pub kind: BrushKind,
    pub color: ColorRgba,
    pub base_w: f32,
    pub opacity: f32,
    pub min_press: f32,
    pub max_press: f32,
    pub smooth: f32,
    pub streamline: f32,
    pub taper_start: f32,
    pub taper_end: f32,
}
```

### Ink points

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PointerKind {
    Pen,
    Touch,
    Mouse,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InkPoint {
    pub x: f32,
    pub y: f32,
    pub t_ms: f64,
    pub press: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub twist: f32,
    pub pointer_type: PointerKind,
}
```

### Strokes

Persist both raw and processed points.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InkStroke {
    pub id: StrokeId,
    pub brush: Brush,
    pub raw_pts: Vec<InkPoint>,
    pub pts: Vec<InkPoint>,
    pub local_bbox: BBox,
    pub world_bbox: BBox,
    pub xform: Xform2D,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}
```

### Layers

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InkLayer {
    pub id: LayerId,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f32,
    pub strokes: Vec<InkStroke>,
}
```

Use `Vec<InkStroke>` to preserve draw order.
Do not use `HashMap<StrokeId, InkStroke>` as canonical storage.

### Document

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InkDoc {
    pub schema_version: u32,
    pub id: DocId,
    pub width: f32,
    pub height: f32,
    pub background: InkBackground,
    pub active_layer_id: LayerId,
    pub layers: Vec<InkLayer>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,

    #[serde(skip)]
    pub runtime: InkRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InkBackground {
    Plain,
    Dots,
    Grid,
    Ruled,
}
```

## Runtime structures

Runtime state is not serialized.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrokeAddress {
    pub layer_idx: usize,
    pub stroke_idx: usize,
}

#[derive(Debug, Default)]
pub struct InkRuntime {
    pub layer_pos: std::collections::HashMap<LayerId, usize>,
    pub stroke_pos: std::collections::HashMap<StrokeId, StrokeAddress>,
    pub stroke_idx: rstar::RTree<IndexedStroke>,
    pub sel_strokes: std::collections::HashSet<StrokeId>,
    pub render_cache: RenderCache,
    pub dirty_regions: Vec<BBox>,
}
```

Spatial index:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct IndexedStroke {
    pub layer_id: LayerId,
    pub stroke_id: StrokeId,
    pub bbox: rstar::AABB<[f32; 2]>,
}
```

Implement:

```rust
impl rstar::RTreeObject for IndexedStroke {
    type Envelope = rstar::AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.bbox
    }
}
```

R-tree indexes whole stroke bounds.
Do not index every point or segment in the first real build.

## Document mutation rules

All mutations must go through `InkDoc` methods.

Required methods:

```rust
impl InkDoc {
    pub fn new(width: f32, height: f32) -> Self;
    pub fn rebuild_runtime(&mut self);

    pub fn active_layer(&self) -> Option<&InkLayer>;
    pub fn active_layer_mut(&mut self) -> Option<&mut InkLayer>;

    pub fn add_stroke(&mut self, layer_id: LayerId, stroke: InkStroke);
    pub fn get_stroke(&self, stroke_id: StrokeId) -> Option<&InkStroke>;
    pub fn get_stroke_mut(&mut self, stroke_id: StrokeId) -> Option<&mut InkStroke>;

    pub fn delete_stroke(&mut self, stroke_id: StrokeId) -> Option<(LayerId, InkStroke)>;
    pub fn delete_strokes(&mut self, stroke_ids: &[StrokeId]) -> Vec<(LayerId, InkStroke)>;

    pub fn clear_layer(&mut self, layer_id: LayerId) -> Vec<InkStroke>;

    pub fn query_bbox(&self, bbox: BBox) -> Vec<StrokeId>;
    pub fn hit_eraser(&self, pos: Point2, radius: f32) -> Vec<StrokeId>;

    pub fn select_lasso(&mut self, polygon: &[Point2]) -> Vec<StrokeId>;
    pub fn clear_sel(&mut self);
}
```

Preserve draw order. Use `Vec::remove` for deletion unless explicit z-order is introduced.
R-tree query returns candidates only. Draw order must still follow document/layer/stroke order.

## Operations and undo/redo

Use reversible transactions.

```rust
#[derive(Debug, Clone)]
pub enum InkOp {
    AddStroke {
        layer_id: LayerId,
        stroke: InkStroke,
    },

    DeleteStrokes {
        strokes: Vec<(LayerId, InkStroke)>,
    },

    TransformStrokes {
        stroke_ids: Vec<StrokeId>,
        before: Vec<Xform2D>,
        after: Vec<Xform2D>,
    },

    SetStrokeBrush {
        stroke_ids: Vec<StrokeId>,
        before: Vec<Brush>,
        after: Brush,
    },

    ClearLayer {
        layer_id: LayerId,
        prev_strokes: Vec<InkStroke>,
    },

    AddLayer {
        layer: InkLayer,
        idx: usize,
    },

    DeleteLayer {
        layer: InkLayer,
        idx: usize,
    },

    ReorderLayer {
        layer_id: LayerId,
        old_idx: usize,
        new_idx: usize,
    },

    SetActiveLayer {
        prev: LayerId,
        next: LayerId,
    },
}

#[derive(Debug, Clone)]
pub struct InkTx {
    pub label: String,
    pub ops: Vec<InkOp>,
}
```

Undo/redo:
- new transaction clears redo stack
- undo applies inverse operations in reverse order
- redo applies operations in original order
- eraser drag becomes one `DeleteStrokes` transaction
- lasso move becomes one `TransformStrokes` transaction

```rust
pub struct UndoRedo {
    pub undo_stack: Vec<InkTx>,
    pub redo_stack: Vec<InkTx>,
    pub max_depth: usize,
}
```

## Session

```rust
pub struct InkSession {
    pub doc: InkDoc,
    pub active_tool: Tool,
    pub active_brush: Brush,
    pub undo_redo: UndoRedo,
    pub dirty: bool,
    pub last_saved_rev: u64,
    pub rev: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tool {
    Pen,
    Pencil,
    Highlighter,
    StrokeEraser,
    Lasso,
    Pan,
    Select,
}
```

Required methods:

```rust
impl InkSession {
    pub fn do_tx(&mut self, tx: InkTx);
    pub fn undo(&mut self);
    pub fn redo(&mut self);

    pub fn add_stroke(&mut self, stroke: InkStroke);
    pub fn erase_at(&mut self, pos: Point2, radius: f32);
    pub fn delete_sel(&mut self);
    pub fn clear_active_layer(&mut self);

    pub fn select_lasso(&mut self, polygon: &[Point2]);
    pub fn move_sel(&mut self, delta: Vec2);

    pub fn export_json(&self) -> Result<String, InkError>;
    pub fn import_json(json: &str) -> Result<Self, InkError>;
    pub fn export_svg(&self) -> Result<String, InkError>;
}
```

## Input state machine

Use an enum.

```rust
pub enum InputState {
    Idle,

    Drawing {
        pointer_id: i32,
        builder: StrokeBuilder,
    },

    Erasing {
        pointer_id: i32,
        erased: Vec<(LayerId, InkStroke)>,
    },

    Lassoing {
        pointer_id: i32,
        polygon: Vec<Point2>,
    },

    Panning {
        pointer_id: i32,
        last_screen_pos: Point2,
    },

    MovingSel {
        pointer_id: i32,
        start_world: Point2,
        last_world: Point2,
        original_xforms: Vec<(StrokeId, Xform2D)>,
    },
}
```

Do not use scattered booleans like `is_drawing`, `is_erasing`, `is_panning`.

## Stroke builder

```rust
pub struct StrokeBuilder {
    pub brush: Brush,
    pub raw_pts: Vec<InkPoint>,
    pub preview_pts: Vec<InkPoint>,
}
```

Required methods:

```rust
impl StrokeBuilder {
    pub fn new(brush: Brush) -> Self;
    pub fn push(&mut self, pt: InkPoint);
    pub fn preview_pts(&self) -> &[InkPoint];
    pub fn finish(self, now_ms: i64) -> Option<InkStroke>;
}
```

Finish pipeline:
1. remove duplicate points
2. resample by distance
3. smooth
4. compute local bbox with brush width
5. set identity xform
6. compute world bbox
7. return stroke

## Viewport

```rust
#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub pan_x: f32,
    pub pan_y: f32,
    pub zoom: f32,
    pub dpr: f32,
    pub screen_w: f32,
    pub screen_h: f32,
}
```

Required methods:
- `screen_to_world`
- `world_to_screen`
- `visible_world_bbox`
- `pan_by_screen_delta`
- `zoom_at_screen_pos`

Store all ink in world coordinates.

## Geometry

Required functions:

```rust
pub fn compute_bbox(pts: &[InkPoint], extra_radius: f32) -> Option<BBox>;
pub fn bbox_intersects(a: BBox, b: BBox) -> bool;
pub fn bbox_contains_point(bbox: BBox, pos: Point2) -> bool;
pub fn distance_to_segment(pos: Point2, a: Point2, b: Point2) -> f32;
pub fn polyline_hit(pts: &[InkPoint], pos: Point2, radius: f32) -> bool;
pub fn point_in_polygon(pos: Point2, polygon: &[Point2]) -> bool;
pub fn polyline_intersects_polygon(pts: &[InkPoint], polygon: &[Point2]) -> bool;
pub fn xform_point(xform: Xform2D, pos: Point2) -> Point2;
pub fn xform_bbox(xform: Xform2D, bbox: BBox) -> BBox;
```

## Rendering

Separate document mutation from rendering.

Canvas renderer should:
- clear screen
- draw paper background
- draw grid/dots
- query R-tree for visible stroke candidates
- render visible layers in document order
- render in-progress stroke
- render selection outlines
- render lasso polygon
- render tool cursor if useful

Never draw directly in R-tree result order.

## Browser behavior

Pointer rules:
- pen draws
- mouse draws only with primary button
- touch pans by default
- touch drawing is optional
- wheel zooms around cursor
- space + drag pans
- lasso selects strokes
- eraser deletes whole strokes
- canvas uses `touch-action: none`

Use PointerEvent data:
- client coordinates
- pressure
- tiltX
- tiltY
- twist
- pointerType
- timeStamp
- coalesced events if available

## UI

Top toolbar:
- Stonepen title
- Pen
- Pencil
- Highlighter
- Eraser
- Lasso
- Pan
- width slider
- color input
- Undo
- Redo
- Clear
- Save
- Load
- Export SVG
- Export PNG

Bottom status bar:
- stroke count
- selected count
- active tool
- zoom
- dirty/saved state
- input mode

Visual style:
- off-white paper
- graphite UI
- muted blue active state
- restrained brass accent
- no emoji
- no glass UI
- no gradient SaaS style

## Testing

`stonepen-core` must include tests for:
- bbox computation
- bbox intersection
- distance to segment
- point in polygon
- stroke hit testing
- lasso selection
- R-tree query
- runtime rebuild
- delete updates index
- undo/redo add stroke
- undo/redo delete strokes
- clear layer undo/redo
- JSON roundtrip
- SVG export contains valid-looking SVG

## Commands

Use these commands:

```sh
cargo fmt
cargo test -p stonepen-core
wasm-pack build crates/stonepen-wasm --target web --out-dir ../../web/pkg
```

Serve `web/` with any static server after WASM build.

## Completion criteria

A task is not complete unless:
- code is formatted
- core tests pass
- WASM builds
- browser app opens
- drawing works
- save/load works
- SVG export works
- PNG export works
- screenshot artifact is produced
- changed files are committed
