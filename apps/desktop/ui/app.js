const invoke = window.__TAURI__?.core?.invoke;
const appWebview = window.__TAURI__?.webview?.getCurrentWebview?.();

const state = {
  root: null,
  scan: null,
  selectedIndex: 0,
  busy: false,
};

const elements = {
  dropZone: document.querySelector("#drop-zone"),
  workspace: document.querySelector("#workspace"),
  chooseButtons: [
    document.querySelector("#choose-folder"),
    document.querySelector("#choose-folder-empty"),
  ],
  folderPath: document.querySelector("#folder-path"),
  clearScan: document.querySelector("#clear-scan"),
  rescan: document.querySelector("#rescan"),
  summary: document.querySelector("#summary"),
  assetCount: document.querySelector("#asset-count"),
  assetList: document.querySelector("#asset-list"),
  assetDetail: document.querySelector("#asset-detail"),
  replaceExisting: document.querySelector("#replace-existing"),
  generationNote: document.querySelector("#generation-note"),
  generate: document.querySelector("#generate"),
  toast: document.querySelector("#toast"),
};

for (const button of elements.chooseButtons) {
  button.addEventListener("click", chooseFolder);
}
elements.rescan.addEventListener("click", () => state.root && scanFolder(state.root));
elements.clearScan.addEventListener("click", clearScan);
elements.generate.addEventListener("click", generateSettings);

initializeDragDrop();

async function initializeDragDrop() {
  if (!appWebview?.onDragDropEvent) {
    showToast("Folder drag-and-drop is unavailable. Use Choose Folder instead.");
    return;
  }

  try {
    await appWebview.onDragDropEvent((event) => {
      if (event.payload.type === "enter" || event.payload.type === "over") {
        elements.dropZone.classList.add("is-dragging");
      } else if (event.payload.type === "leave") {
        elements.dropZone.classList.remove("is-dragging");
      } else if (event.payload.type === "drop") {
        elements.dropZone.classList.remove("is-dragging");
        const folder = event.payload.paths?.[0];
        if (folder) scanFolder(folder);
      }
    });
  } catch (error) {
    showToast(`Could not enable folder drag-and-drop: ${String(error)}`);
  }
}

async function chooseFolder() {
  if (!invoke || state.busy) return;
  const folder = await invoke("choose_export_folder");
  if (folder) await scanFolder(folder);
}

async function scanFolder(folder) {
  if (!invoke || state.busy) return;
  setBusy(true, "Scanning FBX and texture files…");
  try {
    const scan = await invoke("scan_export_folder", { path: folder });
    state.root = scan.root;
    state.scan = scan;
    state.selectedIndex = 0;
    render();
  } catch (error) {
    showToast(String(error));
  } finally {
    setBusy(false);
  }
}

async function generateSettings() {
  if (!invoke || !state.root || state.busy) return;
  const replaceExisting = elements.replaceExisting.checked;
  setBusy(true, "Generating settings files…");
  try {
    const report = await invoke("generate_settings", {
      path: state.root,
      replaceExisting,
    });
    const counts = report.items.reduce((result, item) => {
      result[item.action] = (result[item.action] || 0) + 1;
      return result;
    }, {});
    const summary = [
      counts.generated ? `${counts.generated} generated` : null,
      counts.replaced ? `${counts.replaced} replaced` : null,
      counts.skippedExisting ? `${counts.skippedExisting} preserved` : null,
      counts.skippedInvalid ? `${counts.skippedInvalid} unresolved` : null,
      counts.failed ? `${counts.failed} failed` : null,
    ]
      .filter(Boolean)
      .join(", ");
    showToast(summary || "No settings files required changes.");
    await scanFolder(state.root);
  } catch (error) {
    showToast(String(error));
  } finally {
    setBusy(false);
  }
}

function clearScan() {
  if (state.busy) return;
  state.root = null;
  state.scan = null;
  state.selectedIndex = 0;
  elements.replaceExisting.checked = false;
  elements.workspace.classList.add("is-hidden");
  elements.dropZone.classList.remove("is-hidden", "is-dragging");
  elements.folderPath.textContent = "";
  elements.summary.replaceChildren();
  elements.assetList.replaceChildren();
  elements.assetDetail.replaceChildren();
  elements.assetCount.textContent = "";
  elements.generationNote.textContent = "";
  elements.generate.disabled = true;
}

function setBusy(busy, note = "") {
  state.busy = busy;
  elements.chooseButtons.forEach((button) => (button.disabled = busy));
  elements.clearScan.disabled = busy;
  elements.rescan.disabled = busy;
  elements.generate.disabled = busy || !state.scan;
  elements.generationNote.textContent = note;
}

function render() {
  const scan = state.scan;
  elements.dropZone.classList.add("is-hidden");
  elements.workspace.classList.remove("is-hidden");
  elements.folderPath.textContent = scan.root;
  elements.assetCount.textContent = scan.assets.length;

  const allIssues = [
    ...scan.globalIssues,
    ...scan.assets.flatMap((asset) => asset.issues),
  ];
  const warningCount = allIssues.filter((issue) => issue.severity === "warning").length;
  const errorCount = allIssues.filter((issue) => issue.severity === "error").length;
  const mappingCount = scan.assets.reduce(
    (total, asset) => total + asset.settings.entries.length,
    0,
  );
  const readyCount = scan.assets.filter((asset) => asset.settings.canGenerate).length;

  elements.summary.innerHTML = [
    summaryCard(scan.assets.length, "Asset folders"),
    summaryCard(mappingCount, "Texture redirects"),
    summaryCard(warningCount, "Warnings", warningCount ? "warning" : ""),
    summaryCard(errorCount, "Blocking errors", errorCount ? "error" : ""),
  ].join("");

  elements.assetList.innerHTML = scan.assets
    .map((asset, index) => assetRow(asset, index))
    .join("");
  elements.assetList.querySelectorAll(".asset-row").forEach((button) => {
    button.addEventListener("click", () => {
      state.selectedIndex = Number(button.dataset.index);
      renderAssetListSelection();
      renderAssetDetail();
    });
  });

  renderAssetDetail();
  elements.generationNote.textContent = `${readyCount} of ${scan.assets.length} assets ready`;
  elements.generate.disabled = state.busy || !readyCount;
}

function renderAssetListSelection() {
  elements.assetList.querySelectorAll(".asset-row").forEach((row, index) => {
    row.classList.toggle("is-selected", index === state.selectedIndex);
  });
}

function renderAssetDetail() {
  const asset = state.scan?.assets[state.selectedIndex];
  if (!asset) {
    elements.assetDetail.innerHTML = '<div class="empty-detail">No asset selected</div>';
    return;
  }

  const warningCount = asset.issues.filter((issue) => issue.severity === "warning").length;
  const errorCount = asset.issues.filter((issue) => issue.severity === "error").length;
  const fileRows = asset.files
    .map(
      (file) => `
        <tr>
          <td class="file-name">${escapeHtml(file.path.split(/[\\/]/).pop())}</td>
          <td>${escapeHtml(formatKind(file.kind))}</td>
          <td>${escapeHtml(file.materialNames.join(", ") || "No material")}</td>
        </tr>`,
    )
    .join("");

  const issues =
    asset.issues.length === 0
      ? '<div class="issue">No validation warnings for this asset.</div>'
      : asset.issues
          .map(
            (issue) => `
              <div class="issue ${issue.severity === "error" ? "issue-error" : ""}">
                <strong>${issue.severity === "error" ? "Error" : "Warning"}:</strong>
                ${escapeHtml(issue.message)}
              </div>`,
          )
          .join("");

  elements.assetDetail.innerHTML = `
    <h2 class="detail-title">${escapeHtml(asset.name)}</h2>
    <p class="detail-path">${escapeHtml(asset.folder)}</p>

    <div class="asset-meta">
      <span class="badge ${asset.settings.canGenerate ? "badge-good" : "badge-error"}">
        ${asset.settings.canGenerate ? "Ready to generate" : "Needs review"}
      </span>
      ${warningCount ? `<span class="badge badge-warning">${warningCount} warning${warningCount === 1 ? "" : "s"}</span>` : ""}
      ${errorCount ? `<span class="badge badge-error">${errorCount} error${errorCount === 1 ? "" : "s"}</span>` : ""}
      <span class="badge">${asset.settings.entries.length} redirects</span>
    </div>

    <h3 class="section-title">FBX files</h3>
    <table class="file-table">
      <thead><tr><th>File</th><th>Type</th><th>Materials</th></tr></thead>
      <tbody>${fileRows}</tbody>
    </table>

    <h3 class="section-title">Texture sources</h3>
    <div class="texture-grid">
      ${textureCard("Main + LOD1", asset.mainTextureSet)}
      ${textureCard("LOD2", asset.lod2TextureSet)}
    </div>

    <h3 class="section-title">Validation</h3>
    <div class="issue-list">${issues}</div>

    <h3 class="section-title">settings.json preview</h3>
    <pre class="json-preview">${escapeHtml(asset.settings.json)}</pre>
  `;
}

function assetRow(asset, index) {
  const warnings = asset.issues.filter((issue) => issue.severity === "warning").length;
  const errors = asset.issues.filter((issue) => issue.severity === "error").length;
  return `
    <button class="asset-row ${index === state.selectedIndex ? "is-selected" : ""}" data-index="${index}">
      <strong>${escapeHtml(asset.name)}</strong>
      <div class="asset-meta">
        <span class="badge">${asset.files.length} FBX</span>
        <span class="badge">${asset.settings.entries.length} links</span>
        ${warnings ? `<span class="badge badge-warning">${warnings} ⚠</span>` : ""}
        ${errors ? `<span class="badge badge-error">${errors} errors</span>` : ""}
      </div>
    </button>`;
}

function textureCard(label, textureSet) {
  if (!textureSet) {
    return `
      <div class="texture-card">
        <strong>${label}</strong>
        <span>Not resolved</span>
      </div>`;
  }
  return `
    <div class="texture-card">
      <strong>${label}: ${escapeHtml(textureSet.name)}</strong>
      <span>${textureSet.files.length} maps · ${escapeHtml(textureSet.folder)}</span>
    </div>`;
}

function summaryCard(value, label, status = "") {
  return `
    <div class="summary-card">
      <strong class="${status === "warning" ? "badge-warning" : status === "error" ? "badge-error" : ""}">${value}</strong>
      <span>${label}</span>
    </div>`;
}

function formatKind(kind) {
  const labels = {
    main: "Main",
    lod1: "LOD1",
    lod2: "LOD2",
    window: "Window",
    lod1Window: "LOD1 Window",
    lod2Window: "LOD2 Window",
    milkyWindow: "Milky Window",
    lod1MilkyWindow: "LOD1 Milky Window",
    lod2MilkyWindow: "LOD2 Milky Window",
    glass: "Glass",
    lod1Glass: "LOD1 Glass",
    lod2Glass: "LOD2 Glass",
    grass: "Grass",
    lod1Grass: "LOD1 Grass",
    lod2Grass: "LOD2 Grass",
    water: "Water",
    lod1Water: "LOD1 Water",
    lod2Water: "LOD2 Water",
  };
  return labels[kind] ?? kind;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

let toastTimer;
function showToast(message) {
  clearTimeout(toastTimer);
  elements.toast.textContent = message;
  elements.toast.classList.remove("is-hidden");
  toastTimer = setTimeout(() => elements.toast.classList.add("is-hidden"), 5000);
}
