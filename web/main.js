import init, { mount_stonepen } from "./pkg/stonepen_wasm.js";

await init();
export const stonepen = mount_stonepen("ink-canvas");
