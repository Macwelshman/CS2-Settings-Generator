const invoke = window.__TAURI__?.core?.invoke;
const appWebview = window.__TAURI__?.webview?.getCurrentWebview?.();

const state = {
  root: null,
  scan: null,
  selectedIndex: 0,
  textureOverrides: new Map(),
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
        if (folder) scanFolder(folder, true);
      }
    });
  } catch (error) {
    showToast(`Could not enable folder drag-and-drop: ${String(error)}`);
  }
}

async function chooseFolder() {
  if (!invoke || state.busy) return;
  const folder = await invoke("choose_export_folder");
  if (folder) await scanFolder(folder, true);
}

async function scanFolder(folder, resetOverrides = false) {
  if (!invoke || state.busy) return;
  const selectedFolder = state.scan?.assets[state.selectedIndex]?.folder;
  if (resetOverrides) state.textureOverrides.clear();
  setBusy(true, "Scanning FBX and texture files…");
  try {
    const scan = await invoke("scan_export_folder", {
      path: folder,
      textureOverrides: serializedTextureOverrides(),
    });
    state.root = scan.root;
    state.scan = scan;
    const validFolders = new Set(scan.assets.map((asset) => asset.folder));
    for (const assetFolder of state.textureOverrides.keys()) {
      if (!validFolders.has(assetFolder)) state.textureOverrides.delete(assetFolder);
    }
    const preservedIndex = scan.assets.findIndex((asset) => asset.folder === selectedFolder);
    state.selectedIndex = preservedIndex >= 0 ? preservedIndex : 0;
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
      textureOverrides: serializedTextureOverrides(),
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
  state.textureOverrides.clear();
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
    ${mainTextureEditor(asset)}
    <div class="texture-grid texture-grid-secondary">
      ${textureCard("LOD2", asset.lod2TextureSet)}
    </div>

    <h3 class="section-title">Validation</h3>
    <div class="issue-list">${issues}</div>

    <h3 class="section-title">settings.json preview</h3>
    <pre class="json-preview">${escapeHtml(asset.settings.json)}</pre>
  `;

  bindTextureSourceControls(asset);
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

function mainTextureEditor(asset) {
  const material = mainMaterialForAsset(asset);
  const textureSets = (state.scan?.textureSets ?? [])
    .map((textureSet, index) => ({ textureSet, index }))
    .filter(({ textureSet }) => textureSet.tier === "main");
  const textureOverride = state.textureOverrides.get(asset.folder);
  const selectedIndex = textureOverride
    ? textureSets.find(
        ({ textureSet }) =>
          textureSet.folder === textureOverride.textureSetFolder &&
          textureSet.name === textureOverride.textureSetName,
      )?.index
    : undefined;
  const automaticLabel = "Automatic detection";
  const options = textureSets
    .map(({ textureSet, index }) => {
      const folder = relativeDisplayPath(state.scan.root, textureSet.folder);
      return `<option value="${index}" ${index === selectedIndex ? "selected" : ""}>${escapeHtml(`${textureSet.name} — ${folder}`)}</option>`;
    })
    .join("");
  const applyTargets = materialApplyTargets(asset, material, asset.mainTextureSet);
  const sourceMode = textureOverride ? "Manual selection" : "Automatic";
  const sourceDetails = asset.mainTextureSet
    ? `${asset.mainTextureSet.files.length} maps · ${asset.mainTextureSet.folder}`
    : "No texture set is currently available for the main mesh and LOD1.";
  const applyLabel = !material
    ? "No main material available"
    : !asset.mainTextureSet
      ? "Select a texture set before applying by material"
      : applyTargets.length
        ? `Apply to ${applyTargets.length} other asset${applyTargets.length === 1 ? "" : "s"} using “${material}”`
        : `All eligible assets using “${material}” already match`;

  return `
    <div class="texture-editor">
      <div class="texture-editor-heading">
        <div>
          <span class="label">FBX material</span>
          <strong>${escapeHtml(material || "No material")}</strong>
        </div>
        <span class="badge ${textureOverride ? "badge-warning" : "badge-good"}">${sourceMode}</span>
      </div>
      <label class="texture-select-label" for="main-texture-set">Main + LOD1 texture set</label>
      <select id="main-texture-set" class="texture-select">
        <option value="" ${selectedIndex === undefined ? "selected" : ""}>${escapeHtml(automaticLabel)}</option>
        ${options}
      </select>
      <div class="texture-source-status ${asset.mainTextureSet ? "" : "is-unresolved"}">
        <strong>${escapeHtml(asset.mainTextureSet?.name || "Not resolved")}</strong>
        <span>${escapeHtml(sourceDetails)}</span>
      </div>
      <div class="texture-editor-actions">
        <p>Local asset-named textures take priority automatically. Manual choices affect only this scan and generation run.</p>
        <button id="apply-texture-by-material" class="button button-quiet" ${applyTargets.length ? "" : "disabled"}>
          ${escapeHtml(applyLabel)}
        </button>
      </div>
    </div>`;
}

function bindTextureSourceControls(asset) {
  const select = elements.assetDetail.querySelector("#main-texture-set");
  select?.addEventListener("change", async () => {
    if (state.busy) return;
    if (select.value === "") {
      state.textureOverrides.delete(asset.folder);
    } else {
      const textureSet = state.scan.textureSets[Number(select.value)];
      if (!textureSet) return;
      state.textureOverrides.set(asset.folder, textureOverrideFor(asset, textureSet));
    }
    await scanFolder(state.root);
  });

  const applyButton = elements.assetDetail.querySelector("#apply-texture-by-material");
  applyButton?.addEventListener("click", async () => {
    if (state.busy || !asset.mainTextureSet) return;
    const material = mainMaterialForAsset(asset);
    const targets = materialApplyTargets(asset, material, asset.mainTextureSet);
    for (const target of targets) {
      state.textureOverrides.set(
        target.folder,
        textureOverrideFor(target, asset.mainTextureSet),
      );
    }
    await scanFolder(state.root);
    showToast(
      `Applied ${asset.mainTextureSet.name} to ${targets.length} asset${targets.length === 1 ? "" : "s"} using ${material}.`,
    );
  });
}

function textureOverrideFor(asset, textureSet) {
  return {
    assetFolder: asset.folder,
    textureSetFolder: textureSet.folder,
    textureSetName: textureSet.name,
  };
}

function serializedTextureOverrides() {
  return [...state.textureOverrides.values()];
}

function mainMaterialForAsset(asset) {
  return asset.files.find((file) => file.kind === "main")?.materialNames?.[0] ?? null;
}

function materialApplyTargets(asset, material, textureSet) {
  if (!material || !textureSet || !state.scan) return [];
  return state.scan.assets.filter((candidate) => {
    if (candidate.folder === asset.folder) return false;
    if (mainMaterialForAsset(candidate) !== material) return false;
    if (isLocalExactTextureSet(candidate)) return false;
    if (
      candidate.mainTextureSet?.folder === textureSet.folder &&
      candidate.mainTextureSet?.name === textureSet.name
    ) {
      return false;
    }
    const currentOverride = state.textureOverrides.get(candidate.folder);
    return !(
      currentOverride?.textureSetFolder === textureSet.folder &&
      currentOverride?.textureSetName === textureSet.name
    );
  });
}

function isLocalExactTextureSet(asset) {
  return (
    asset.mainTextureSet?.folder === asset.folder &&
    asset.mainTextureSet?.name === asset.name
  );
}

function relativeDisplayPath(root, folder) {
  if (!root || !folder.startsWith(root)) return folder;
  const relative = folder.slice(root.length).replace(/^[/\\]+/, "");
  return relative || ".";
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
