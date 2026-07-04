import init, { start_stonepen } from "./pkg/stonepen_wasm.js";

await init();
start_stonepen("ink-canvas");
