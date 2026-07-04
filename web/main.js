import init, { WasmApp } from "./pkg/stonepen_wasm.js";

async function main() {
  await init();

  const app = new WasmApp("ink-canvas");
  const canvas = document.getElementById("ink-canvas");

  canvas.addEventListener("pointerdown", (e) => {
    e.preventDefault();
    canvas.focus();
    app.on_pointer_down(e);
  });
  canvas.addEventListener("pointermove", (e) => { e.preventDefault(); app.on_pointer_move(e); });
  canvas.addEventListener("pointerup", (e) => { e.preventDefault(); app.on_pointer_up(e); });
  canvas.addEventListener("pointercancel", (e) => { app.on_pointer_cancel(e); });
  canvas.addEventListener("wheel", (e) => { e.preventDefault(); app.on_wheel(e); }, { passive: false });
  window.addEventListener("keydown", (e) => {
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
    app.on_key(e);
  });

  canvas.addEventListener("redraw", () => {
    app.redraw();
  });

  window.addEventListener("paste", (e) => {
    const items = e.clipboardData?.items;
    if (!items) return;
    for (let i = 0; i < items.length; i++) {
      const item = items[i];
      if (item.type.indexOf("image") !== -1) {
        const file = item.getAsFile();
        const reader = new FileReader();
        reader.onload = (ev) => {
          const bytes = new Uint8Array(ev.target.result);
          const img = new Image();
          img.onload = () => {
            app.paste_image(bytes, file.type, img.width, img.height);
          };
          img.src = URL.createObjectURL(file);
        };
        reader.readAsArrayBuffer(file);
        e.preventDefault();
        break;
      }
    }
  });

  const toolBtns = document.querySelectorAll(".tool-btn");
  toolBtns.forEach((btn) => {
    btn.addEventListener("click", () => {
      toolBtns.forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");
      const tool = btn.dataset.tool;
      app.set_tool(tool);
      canvas.className = "";
      if (tool === "pan") canvas.classList.add("tool-pan");
      else if (tool === "eraser") canvas.classList.add("tool-eraser");
      else if (tool === "lasso") canvas.classList.add("tool-lasso");
      else if (tool === "select") canvas.classList.add("tool-select");
    });
  });

  document.getElementById("width-slider").addEventListener("input", (e) => {
    app.set_brush_width(parseFloat(e.target.value));
  });

  document.getElementById("color-picker").addEventListener("input", (e) => {
    const hex = e.target.value;
    const r = parseInt(hex.slice(1, 3), 16);
    const g = parseInt(hex.slice(3, 5), 16);
    const b = parseInt(hex.slice(5, 7), 16);
    app.set_brush_color(r, g, b);
  });

  document.getElementById("btn-undo").addEventListener("click", () => { app.action_undo(); });
  document.getElementById("btn-redo").addEventListener("click", () => { app.action_redo(); });
  document.getElementById("btn-clear").addEventListener("click", () => { app.action_clear(); });
  document.getElementById("btn-save").addEventListener("click", () => { app.action_save(); });
  document.getElementById("btn-export-svg").addEventListener("click", () => { app.action_export_svg(); });
  document.getElementById("btn-export-png").addEventListener("click", () => { app.action_export_png(); });

  const loadInput = document.getElementById("load-input");
  document.getElementById("btn-load").addEventListener("click", () => { loadInput.click(); });
  loadInput.addEventListener("change", () => {
    const file = loadInput.files[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = (ev) => { app.action_load(ev.target.result); };
    reader.readAsText(file);
    loadInput.value = "";
  });

  const ro = new ResizeObserver(() => { app.resize(); });
  ro.observe(canvas);

  app.redraw();
}

main().catch(console.error);
