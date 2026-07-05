import init, { mount_stonepen } from "./pkg/stonepen_wasm.js";

await init();
const stonepen = mount_stonepen("ink-canvas");
window.stonepen = stonepen;
