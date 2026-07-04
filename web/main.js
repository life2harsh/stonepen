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
  const isEditing = (el) => {
    if (!el) return false;
    if (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.tagName === "SELECT") {
      return true;
    }
    if (el.isContentEditable) {
      return true;
    }
    return false;
  };

  window.addEventListener("keydown", (e) => {
    if (app.is_capturing()) {
      e.preventDefault();
      app.on_key_down(e);
      return;
    }
    if (isEditing(e.target)) return;
    app.on_key_down(e);
  });

  window.addEventListener("keyup", (e) => {
    if (app.is_capturing()) {
      e.preventDefault();
      app.on_key_up(e);
      return;
    }
    if (isEditing(e.target)) return;
    app.on_key_up(e);
  });

  window.addEventListener("blur", () => {
    app.on_blur();
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
      app.set_tool(btn.dataset.tool);
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

  // Settings UI logic
  const settingsModal = document.getElementById("settings-modal");
  const btnSettings = document.getElementById("btn-settings");
  const btnSettingsClose = document.getElementById("btn-settings-close");
  const btnSettingsCloseFooter = document.getElementById("btn-settings-close-footer");
  const btnSettingsReset = document.getElementById("btn-settings-reset");
  const tableContainer = document.getElementById("shortcuts-table-container");
  const captureOverlay = document.getElementById("capture-overlay");
  const captureCmdName = document.getElementById("capture-cmd-name");

  const renderShortcuts = () => {
    const shortcuts = JSON.parse(app.get_shortcuts_json());
    tableContainer.innerHTML = "";

    const groups = [
      { name: "Tools", commandIds: ["tool_pen", "tool_pencil", "tool_highlighter", "tool_eraser", "tool_lasso", "tool_select", "tool_pan"] },
      { name: "Actions", commandIds: ["undo", "redo", "delete_selection", "duplicate_selection", "clear_selection"] },
      { name: "Navigation", commandIds: ["hold_pan"] },
    ];

    groups.forEach((group) => {
      const h4 = document.createElement("h4");
      h4.textContent = group.name;
      h4.style.margin = "12px 0 6px 0";
      h4.style.color = "var(--title-color)";
      tableContainer.appendChild(h4);

      const groupRows = shortcuts.filter(row => group.commandIds.includes(row.command_id));
      groupRows.forEach((row) => {
        const div = document.createElement("div");
        div.className = "shortcut-row";

        const label = document.createElement("div");
        label.className = "shortcut-label";
        label.textContent = row.label;
        div.appendChild(label);

        const bindingsDiv = document.createElement("div");
        bindingsDiv.className = "shortcut-bindings";
        row.bindings.forEach((binding, idx) => {
          const badge = document.createElement("span");
          badge.className = "shortcut-badge";
          badge.textContent = binding;

          const removeBtn = document.createElement("button");
          removeBtn.className = "shortcut-badge-remove";
          removeBtn.innerHTML = "&times;";
          removeBtn.addEventListener("click", (e) => {
            e.stopPropagation();
            app.remove_shortcut_binding(row.command_id, idx);
          });
          badge.appendChild(removeBtn);
          bindingsDiv.appendChild(badge);
        });
        div.appendChild(bindingsDiv);

        const actionsDiv = document.createElement("div");
        actionsDiv.className = "shortcut-actions";

        const addBtn = document.createElement("button");
        addBtn.className = "add-binding-btn";
        addBtn.textContent = row.bindings.length > 0 ? "Add..." : "Bind...";
        addBtn.addEventListener("click", () => {
          app.start_capture(row.command_id);
        });
        actionsDiv.appendChild(addBtn);
        div.appendChild(actionsDiv);

        tableContainer.appendChild(div);
      });
    });

    if (app.is_capturing()) {
      captureCmdName.textContent = app.capturing_label();
      captureOverlay.classList.remove("hidden");
    } else {
      captureOverlay.classList.add("hidden");
    }
  };

  btnSettings.addEventListener("click", () => {
    renderShortcuts();
    settingsModal.classList.add("show");
  });

  const closeModal = () => {
    app.cancel_capture();
    settingsModal.classList.remove("show");
  };

  btnSettingsClose.addEventListener("click", closeModal);
  btnSettingsCloseFooter.addEventListener("click", closeModal);

  btnSettingsReset.addEventListener("click", () => {
    if (confirm("Are you sure you want to reset all keyboard shortcuts to defaults?")) {
      app.reset_shortcuts_to_defaults();
    }
  });

  window.addEventListener("shortcuts-updated", () => {
    renderShortcuts();
  });

  window.addEventListener("shortcut-conflict", (e) => {
    alert(e.detail);
  });

  const ro = new ResizeObserver(() => { app.resize(); });
  ro.observe(canvas);

  app.redraw();
}

main().catch(console.error);
