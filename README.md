# Stonepen

Standalone Rust/WASM vector handwriting input engine.

Stonepen captures handwritten ink as editable vector strokes. It does not interpret, classify, OCR, or convert handwriting into text.

## Workspace

- `crates/stonepen-core` — core engine: document model, geometry, spatial index, undo/redo, export

## Build

```sh
cargo test -p stonepen-core
```

## License

MIT
