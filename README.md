# Stonepen

Standalone Rust/WASM vector handwriting input engine.

Stonepen captures handwritten ink as editable vector strokes. It does not interpret, classify, OCR, or convert handwriting into text.

## Workspace

- `crates/stonepen-core` — core engine: document model, geometry, spatial index, undo/redo, export

## Build

To run the complete verification check and build the WASM package:

```sh
./scripts/build-web.sh
```

To run unit tests directly:

```sh
cargo test -p stonepen-core
```

To build the WASM package directly:

```sh
wasm-pack build crates/stonepen-wasm --target web --out-dir ../../web/pkg
```

## License

MIT
