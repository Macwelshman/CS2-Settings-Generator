const invoke = window.__TAURI__?.core?.invoke;
const appWebview = window.__TAURI__?.webview?.getCurrentWebview?.();

const state = {
  root: null,
  scan: null,
  selectedIndex: 0,
  textureOverrides: new Map(),
  projectMain: null,
  lod2Overrides: new Map(),
  assetSettingsOverrides: new Map(),
  busy: false,
  recoveryWarnings: [],
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
  projectTextures: document.querySelector("#project-textures"),
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
const restoreButton = document.querySelector("#restore-project");
const relocateButton = document.querySelector("#relocate-project");
const lastProjectKey = "cs2-settings:last-project";
const profileKey = (root) => `cs2-settings:project:${root}`;
restoreButton.disabled = relocateButton.disabled = !localStorage.getItem(lastProjectKey);
const importButton = document.querySelector("#import-profile");
const exportButton = document.querySelector("#export-profile");
importButton.addEventListener("click", async () => {
  if (!invoke || state.busy) return;
  let folder, profile;
  setBusy(true);
  try {
    profile = await invoke("open_project_profile");
    if (profile !== null) {
      JSON.parse(profile);
      folder = await invoke("choose_export_folder");
    }
  } catch (error) {
    showToast(`Could not open project profile: ${String(error)}`);
  } finally {
    setBusy(false);
  }
  if (folder) await scanFolder(folder, true, profile, true);
});
exportButton.addEventListener("click", async () => {
  if (!invoke || !state.root || state.busy || state.recoveryWarnings.length) return;
  setBusy(true);
  try {
    const contents = JSON.stringify(ProjectProfiles.encode(state.root, projectChoices(), state.scan), null, 2) + "\n";
    if (await invoke("export_project_profile", { contents })) showToast("Project profile exported.");
  } catch (error) {
    showToast(String(error));
  } finally {
    setBusy(false);
  }
});
relocateButton.addEventListener("click", async () => {
  if (!invoke || state.busy) return;
  const oldRoot = localStorage.getItem(lastProjectKey);
  const saved = oldRoot && localStorage.getItem(profileKey(oldRoot));
  if (!saved) return;
  const folder = await invoke("choose_export_folder");
  if (folder) await scanFolder(folder, true, saved);
});
restoreButton.addEventListener("click", () => {
  const root = localStorage.getItem(lastProjectKey);
  if (root) scanFolder(root, true);
});
elements.rescan.addEventListener("click", () => state.root && scanFolder(state.root));
elements.clearScan.addEventListener("click", clearScan);
elements.generate.addEventListener("click", generateSettings);

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

async function scanFolder(folder, resetOverrides = false, relocatedProfile = null, importing = false) {
  if (!invoke || state.busy) return;
  const selectedFolder = state.scan?.assets[state.selectedIndex]?.folder;
  setBusy(true, "Scanning FBX and texture files…");
  try {
    const scanRequest = (path, choices) => invoke("scan_export_folder", {
      path,
      textureOverrides: choices.textureOverrides,
      textureOptions: { projectMain: choices.projectMain, lod2Overrides: choices.lod2Overrides },
      assetSettingsOverrides: choices.assetSettingsOverrides,
    });
    const empty = ProjectProfiles.empty();
    const initial = await scanRequest(folder, empty);
    const existingProfile = resetOverrides ? localStorage.getItem(profileKey(initial.root)) : null;
    if (!importing && relocatedProfile && existingProfile !== null && existingProfile !== relocatedProfile) {
      throw new Error("That folder already has saved choices. Open it with Scan Export Folder.");
    }
    const saved = relocatedProfile ?? existingProfile;
    let expectedTextures, expectedFbx;
    let choices = resetOverrides ? empty : projectChoices();
    let recoveryWarnings = resetOverrides ? [] : state.recoveryWarnings;
    if (saved !== null) {
      try {
        ({ choices, warnings: recoveryWarnings, expectedTextures, expectedFbx } = ProjectProfiles.decode(
          JSON.parse(saved), initial.root, !importing && !relocatedProfile,
        ));
      } catch (error) {
        if (importing || relocatedProfile) throw error;
        recoveryWarnings = [String(error)];
      }
    }
    const active = activeChoices(choices, initial.assets);
    const scan = active.projectMain || active.textureOverrides.length || active.lod2Overrides.length || active.assetSettingsOverrides.length
      ? await scanRequest(initial.root, active) : initial;
    const findings = ProjectProfiles.findings(choices, scan, { expectedTextures, expectedFbx });
    if (importing && findings.length) {
      throw new Error(`Profile was not imported: ${findings.join(" ")}`);
    }
    if (importing && existingProfile !== null && !(await invoke("confirm_profile_replacement"))) return;
    if (importing || (relocatedProfile && existingProfile === null)) {
      localStorage.setItem(profileKey(scan.root), JSON.stringify(ProjectProfiles.encode(scan.root, choices)));
    }
    state.recoveryWarnings = recoveryWarnings;
    for (const message of recoveryWarnings) {
      scan.globalIssues.push({ severity: "error", path: scan.root,
        message: `Saved choices were partially recovered. The original configuration was preserved; import a valid profile to replace it. ${message}` });
    }
    for (const message of findings) {
      scan.globalIssues.push({ severity: "warning", path: scan.root, message });
    }
    state.root = scan.root;
    state.scan = scan;
    state.projectMain = choices.projectMain;
    for (const key of ["textureOverrides", "lod2Overrides", "assetSettingsOverrides"]) {
      state[key] = new Map(choices[key].map((item) => [item.assetFolder, item]));
    }
    if (resetOverrides) elements.replaceExisting.checked = false;
    const missing = ["textureOverrides", "lod2Overrides", "assetSettingsOverrides"]
      .flatMap((key) => choices[key].filter((item) => !active[key].includes(item)));
    for (const item of new Map(missing.map((item) => [item.assetFolder, item])).values()) {
      scan.globalIssues.push({ severity: "warning", path: item.assetFolder,
        message: `Saved choices for “${item.assetName}” were kept but not applied. Restore the original FBX name and folder, or discard the stale choices before configuring the new asset.` });
      for (const asset of scan.assets.filter((asset) => asset.folder === item.assetFolder)) {
        asset.settings.canGenerate = false;
        asset.issues.push({ severity: "error", code: "renamedAsset", message: "Review the saved choices for the renamed asset before generating." });
      }
    }
    if (!recoveryWarnings.length && (!resetOverrides || existingProfile === null) && !importing && !relocatedProfile) saveProject();
    else localStorage.setItem(lastProjectKey, scan.root);
    const preservedIndex = scan.assets.findIndex((asset) => asset.folder === selectedFolder);
    state.selectedIndex = preservedIndex >= 0 ? preservedIndex : 0;
    render();
    if (importing) showToast("Project profile imported.");
  } catch (error) {
    showToast(importing ? String(error) : `Could not restore or scan project: ${String(error)}`);
  } finally {
    setBusy(false);
  }
}

function savedChoice(map, asset) {
  const choice = map.get(asset.folder);
  return choice?.assetName === asset.name ? choice : undefined;
}

function projectChoices() {
  return {
    textureOverrides: serializedTextureOverrides(),
    lod2Overrides: [...state.lod2Overrides.values()],
    assetSettingsOverrides: serializedAssetSettingsOverrides(),
    projectMain: state.projectMain,
  };
}

function activeChoices(choices, assets) {
  const active = { ...choices };
  for (const key of ["textureOverrides", "lod2Overrides", "assetSettingsOverrides"]) {
    active[key] = ProjectProfiles.partition(choices[key], assets);
  }
  return active;
}

function saveProject() {
  try {
    localStorage.setItem(profileKey(state.root), JSON.stringify(ProjectProfiles.encode(state.root, projectChoices())));
    localStorage.setItem(lastProjectKey, state.root);
  } catch (error) {
    showToast(`Could not save project configuration: ${String(error)}`);
  }
}

async function generateSettings() {
  if (!invoke || !state.root || state.busy || state.recoveryWarnings.length) return;
  const choices = projectChoices();
  const active = activeChoices(choices, state.scan.assets);
  if (["textureOverrides", "lod2Overrides", "assetSettingsOverrides"].some((key) =>
    choices[key].some((item) => !active[key].includes(item) && state.scan.assets.some((asset) => asset.folder === item.assetFolder)))) {
    showToast("Resolve saved choices for renamed assets before generating.");
    return;
  }
  const replaceExisting = elements.replaceExisting.checked;
  setBusy(true, "Generating settings files…");
  try {
    const report = await invoke("generate_settings", {
      path: state.root,
      replaceExisting,
      reviewed: state.scan.assets.map(({ name, folder, settings }) => ({
        name, folder, json: settings.json, canGenerate: settings.canGenerate,
        existingFile: settings.existingFile, existingRevision: settings.existingRevision,
      })),
      textureOverrides: active.textureOverrides,
      textureOptions: { projectMain: active.projectMain, lod2Overrides: active.lod2Overrides },
      assetSettingsOverrides: active.assetSettingsOverrides,
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
    setBusy(false);
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
  state.recoveryWarnings = [];
  exportButton.disabled = true;
  state.scan = null;
  state.selectedIndex = 0;
  state.textureOverrides.clear();
  state.projectMain = null;
  state.lod2Overrides.clear();
  state.assetSettingsOverrides.clear();
  elements.replaceExisting.checked = false;
  elements.workspace.classList.add("is-hidden");
  elements.dropZone.classList.remove("is-hidden", "is-dragging");
  elements.folderPath.textContent = "";
  elements.summary.replaceChildren();
  document.querySelector("#project-findings")?.remove();
  elements.assetList.replaceChildren();
  elements.assetDetail.replaceChildren();
  elements.assetCount.textContent = "";
  elements.generationNote.textContent = "";
  elements.generate.disabled = true;
}

function setBusy(busy, note = "") {
  state.busy = busy;
  restoreButton.disabled = relocateButton.disabled = busy || !localStorage.getItem(lastProjectKey);
  importButton.disabled = busy;
  exportButton.disabled = busy || !state.root || state.recoveryWarnings.length > 0;
  elements.chooseButtons.forEach((button) => (button.disabled = busy));
  elements.clearScan.disabled = busy;
  elements.rescan.disabled = busy;
  elements.generate.disabled = busy || state.recoveryWarnings.length > 0 || !state.scan?.assets.some((asset) => asset.settings.canGenerate);
  elements.workspace.querySelectorAll("select").forEach((select) => (select.disabled = busy || state.recoveryWarnings.length > 0));
  if (state.recoveryWarnings.length) {
    elements.workspace.querySelectorAll("[data-asset-type], #discard-stale, #apply-texture-by-material, #normal-opacity-enabled, #normal-opacity")
      .forEach((control) => (control.disabled = true));
  }
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

  document.querySelector("#project-findings")?.remove();
  if (scan.globalIssues.length) {
    elements.summary.insertAdjacentHTML("afterend", `<details id="project-findings"><summary>Project import readiness (${scan.globalIssues.length})</summary>
      <div class="issue-list">${scan.globalIssues.map((issue) => `<div class="issue"><strong>${escapeHtml(issue.severity)}:</strong>
      ${escapeHtml(issue.message)} <small>${escapeHtml(issue.path ?? "")}</small></div>`).join("")}</div></details>`);
  }
  renderProjectTextures();
  renderAssetDetail();
  elements.generationNote.textContent = `${readyCount} of ${scan.assets.length} assets ready`;
  elements.generate.disabled = state.busy || state.recoveryWarnings.length > 0 || !readyCount;
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
          <td>${escapeHtml(file.meshNames.join(", ") || "No mesh object")}</td>
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
                ${escapeHtml(issue.message)} ${issue.path ? `<small>${escapeHtml(issue.path.split(/[\\/]/).pop())}</small>` : ""}
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
      <span class="badge ${asset.assetType === "decal" ? "badge-decal" : ""}">${asset.assetType === "decal" ? "Decal" : "Standard asset"}</span>
    </div>

    <h3 class="section-title">Asset type</h3>
    ${assetTypeEditor(asset)}

    <h3 class="section-title">FBX files</h3>
    <table class="file-table">
      <thead><tr><th>File</th><th>Type</th><th>Mesh object</th><th>Materials</th></tr></thead>
      <tbody>${fileRows}</tbody>
    </table>

    <h3 class="section-title">Texture sources</h3>
    ${mainTextureEditor(asset)}
    ${asset.assetType === "decal" ? "" : `<div class="texture-grid texture-grid-secondary">${lod2TextureEditor(asset)}</div>`}

    <h3 class="section-title">Import Readiness</h3>
    <div class="issue-list">${issues}</div>
    ${asset.issues.some((issue) => issue.code === "renamedAsset") ? '<button id="discard-stale" class="button button-secondary">Discard Choices for Previous Asset</button>' : ""}

    <h3 class="section-title">settings.json preview</h3>
    ${asset.settings.existingFile ? `<details open>
      <summary>Changes Only</summary>
      <pre class="json-preview">${escapeHtml(formatChanges(asset.settings))}</pre>
    </details>` : ""}
    <details ${asset.settings.existingFile ? "" : "open"}>
      <summary>Full JSON</summary>
      <pre class="json-preview">${escapeHtml(asset.settings.json)}</pre>
    </details>
  `;

  elements.assetDetail.querySelector("#discard-stale")?.addEventListener("click", () => {
    if (state.busy || state.recoveryWarnings.length) return;
    for (const map of [state.textureOverrides, state.lod2Overrides, state.assetSettingsOverrides]) {
      if (!savedChoice(map, asset)) map.delete(asset.folder);
    }
    scanFolder(state.root);
  });
  bindAssetTypeControls(asset);
  bindTextureSourceControls(asset);
}

function formatChanges(settings) {
  if (settings.comparisonError) return settings.comparisonError;
  return settings.changes?.map(({ action, path, before, after }) => {
    const lines = [`${action[0].toUpperCase()}${action.slice(1)} ${path || "(document)"}`];
    if (action !== "added") lines.push(`− ${JSON.stringify(before)}`);
    if (action !== "removed") lines.push(`+ ${JSON.stringify(after)}`);
    return lines.join("\n");
  }).join("\n\n") || "No changes.";
}

function assetRow(asset, index) {
  const warnings = asset.issues.filter((issue) => issue.severity === "warning").length;
  const errors = asset.issues.filter((issue) => issue.severity === "error").length;
  return `
    <button class="asset-row ${index === state.selectedIndex ? "is-selected" : ""}" data-index="${index}">
      <strong>${escapeHtml(asset.name)}</strong>
      <div class="asset-meta">
        <span class="badge">${asset.files.length} FBX</span>
        ${asset.assetType === "decal" ? '<span class="badge badge-decal">Decal</span>' : ""}
        <span class="badge">${asset.settings.entries.length} links</span>
        ${warnings ? `<span class="badge badge-warning">${warnings} ⚠</span>` : ""}
        ${errors ? `<span class="badge badge-error">${errors} errors</span>` : ""}
      </div>
    </button>`;
}

function assetTypeEditor(asset) {
  const isDecal = asset.assetType === "decal";
  const hasNormalOpacity = asset.normalOpacity !== null;
  const normalOpacity = hasNormalOpacity ? asset.normalOpacity : 1;
  return `
    <div class="asset-type-editor">
      <div class="asset-type-options" role="group" aria-label="Asset type">
        <button type="button" class="asset-type-option ${isDecal ? "" : "is-selected"}" data-asset-type="standard">Standard asset</button>
        <button type="button" class="asset-type-option ${isDecal ? "is-selected" : ""}" data-asset-type="decal">Decal</button>
      </div>
      ${isDecal ? `
        <p class="asset-type-help">Adds the DefaultDecal material template. Decals use a main mesh and require BaseColor, MaskMap, and Normal textures.</p>
        <label class="normal-opacity-toggle">
          <input id="normal-opacity-enabled" type="checkbox" ${hasNormalOpacity ? "checked" : ""} />
          <span>Override normal opacity</span>
        </label>
        <div class="normal-opacity-control ${hasNormalOpacity ? "" : "is-disabled"}">
          <input id="normal-opacity" type="range" min="0" max="1" step="0.05" value="${normalOpacity}" ${hasNormalOpacity ? "" : "disabled"} />
          <output id="normal-opacity-value">${formatOpacity(normalOpacity)}</output>
        </div>
        <p class="asset-type-help">0 lets the underlying surface normal show through; 1 uses the decal normal fully.</p>
        <div class="decal-after-import"><strong>After importing</strong><span>Texture Area, Render Priority, Layer Mask, and Infoview Color remain editable on the decal Render Prefab in CS2.</span></div>
      ` : '<p class="asset-type-help">Uses the standard mesh and LOD texture-sharing rules.</p>'}
    </div>`;
}

function bindAssetTypeControls(asset) {
  elements.assetDetail.querySelectorAll("[data-asset-type]").forEach((button) => {
    button.addEventListener("click", async () => {
      if (state.busy || state.recoveryWarnings.length || button.dataset.assetType === asset.assetType) return;
      if (button.dataset.assetType === "standard") {
        state.assetSettingsOverrides.delete(asset.folder);
      } else {
        state.assetSettingsOverrides.set(asset.folder, {
          assetFolder: asset.folder,
          assetName: asset.name,
          assetType: "decal",
          normalOpacity: null,
        });
      }
      await scanFolder(state.root);
    });
  });

  const enabled = elements.assetDetail.querySelector("#normal-opacity-enabled");
  const slider = elements.assetDetail.querySelector("#normal-opacity");
  const output = elements.assetDetail.querySelector("#normal-opacity-value");
  slider?.addEventListener("input", () => {
    if (output) output.textContent = formatOpacity(Number(slider.value));
  });
  enabled?.addEventListener("change", async () => {
    if (state.busy || state.recoveryWarnings.length) return;
    const current = savedChoice(state.assetSettingsOverrides, asset) ?? {
      assetFolder: asset.folder,
      assetName: asset.name,
      assetType: "decal",
      normalOpacity: null,
    };
    current.normalOpacity = enabled.checked ? Number(slider?.value ?? 1) : null;
    state.assetSettingsOverrides.set(asset.folder, current);
    await scanFolder(state.root);
  });
  slider?.addEventListener("change", async () => {
    if (state.busy || state.recoveryWarnings.length || !enabled?.checked) return;
    state.assetSettingsOverrides.set(asset.folder, {
      assetFolder: asset.folder,
      assetName: asset.name,
      assetType: "decal",
      normalOpacity: Number(slider.value),
    });
    await scanFolder(state.root);
  });
}

function formatOpacity(value) {
  return Number(value).toFixed(2).replace(/0+$/, "").replace(/\.$/, "");
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

function selectionOptions(tier, selection) {
  const sets = state.scan.textureSets.map((set, index) => ({ set, index })).filter(({ set }) => set.tier === tier);
  const matches = (set) => selection && set.folder === selection.textureSetFolder && set.name === selection.textureSetName;
  const unavailable = selection && !sets.some(({ set }) => matches(set));
  return (unavailable ? `<option value="missing" selected>Unavailable: ${escapeHtml(selection.textureSetName)}</option>` : "") + sets.map(({ set, index }) =>
    `<option value="${index}" ${matches(set) ? "selected" : ""}>${escapeHtml(`${set.name} — ${relativeDisplayPath(state.scan.root, set.folder)}`)}</option>`
  ).join("");
}

function renderProjectTextures() {
  elements.projectTextures.innerHTML = `
    <label class="texture-select-label" for="project-main-texture-set">Project main texture set</label>
    <select id="project-main-texture-set" class="texture-select">
      <option value="" ${state.projectMain ? "" : "selected"}>Automatic detection per asset</option>
      ${selectionOptions("main", state.projectMain)}
    </select>
    <p class="texture-resolution-help">Choose once for the whole project. Each asset’s main mesh and LOD1 use this set unless you make a manual selection below. Choices are saved for this project on this computer.</p>`;
  elements.projectTextures.querySelector("select").addEventListener("change", async (event) => {
    if (state.busy || state.recoveryWarnings.length) return;
    if (event.target.value === "missing") return;
    const set = event.target.value === "" ? null : state.scan.textureSets[Number(event.target.value)];
    state.projectMain = set ? { textureSetFolder: set.folder, textureSetName: set.name } : null;
    await scanFolder(state.root);
  });
}

function lod2TextureEditor(asset) {
  if (!asset.files.some((file) => file.kind === "lod2")) return textureCard("LOD2", null);
  const selection = savedChoice(state.lod2Overrides, asset);
  return `<div class="texture-editor">
    <label class="texture-select-label" for="lod2-texture-set">LOD2 texture set</label>
    <select id="lod2-texture-set" class="texture-select">
      <option value="" ${selection ? "" : "selected"}>Automatic detection (local, then parent folder)</option>
      ${selectionOptions("lod2", selection)}
    </select>
    <span class="badge">${selection ? "Manual selection" : "Automatic"}</span>
    ${textureCard("LOD2", asset.lod2TextureSet)}
  </div>`;
}

function mainTextureEditor(asset) {
  const material = mainMaterialForAsset(asset);
  const textureSets = (state.scan?.textureSets ?? [])
    .map((textureSet, index) => ({ textureSet, index }))
    .filter(({ textureSet }) => textureSet.tier === "main");
  const textureOverride = savedChoice(state.textureOverrides, asset);
  const selectedIndex = textureOverride
    ? textureSets.find(
        ({ textureSet }) =>
          textureSet.folder === textureOverride.textureSetFolder &&
          textureSet.name === textureOverride.textureSetName,
      )?.index
    : undefined;
  const automaticLabel = state.projectMain ? "Use project selection" : "Automatic detection";
  const options = textureSets
    .map(({ textureSet, index }) => {
      const folder = relativeDisplayPath(state.scan.root, textureSet.folder);
      return `<option value="${index}" ${index === selectedIndex ? "selected" : ""}>${escapeHtml(`${textureSet.name} — ${folder}`)}</option>`;
    })
    .join("");
  const applyTargets = materialApplyTargets(asset, material, asset.mainTextureSet);
  const sourceMode = textureOverride ? "Manual selection" : state.projectMain ? "Project selection" : "Automatic";
  const sourceDetails = asset.mainTextureSet
    ? `${asset.mainTextureSet.files.length} maps · ${asset.mainTextureSet.folder}`
    : asset.assetType === "decal"
      ? "No texture set is currently available for this decal."
      : "No texture set is currently available for the main mesh and LOD1.";
  const noTextureHelp = textureSets.length
    ? "Automatic matching could not choose safely. Select a discovered texture set above; it will be used by both the main mesh and LOD1."
    : "No usable PNG texture sets were found inside the project. Add textures to the project or an asset folder and rescan.";
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
      <label class="texture-select-label" for="main-texture-set">${asset.assetType === "decal" ? "Decal texture set" : "Main + LOD1 texture set"}</label>
      <select id="main-texture-set" class="texture-select">
        <option value="" ${selectedIndex === undefined ? "selected" : ""}>${escapeHtml(automaticLabel)}</option>
        ${textureOverride && selectedIndex === undefined ? `<option value="missing" selected>Unavailable: ${escapeHtml(textureOverride.textureSetName)}</option>` : ""}
        ${options}
      </select>
      <div class="texture-source-status ${asset.mainTextureSet ? "" : "is-unresolved"}">
        <strong>${escapeHtml(asset.mainTextureSet?.name || "Not resolved")}</strong>
        <span>${escapeHtml(sourceDetails)}</span>
      </div>
      ${asset.mainTextureSet ? "" : `<p class="texture-resolution-help">${escapeHtml(noTextureHelp)}</p>`}
      <div class="texture-editor-actions">
        <p>Automatic detection matches the FBX material name to the texture set name inside the project. Main and LOD1 share that set. An asset’s manual selection takes priority over the project selection.</p>
        <button id="apply-texture-by-material" class="button button-quiet" ${applyTargets.length ? "" : "disabled"}>
          ${escapeHtml(applyLabel)}
        </button>
      </div>
    </div>`;
}

function bindTextureSourceControls(asset) {
  elements.assetDetail.querySelector("#lod2-texture-set")?.addEventListener("change", async (event) => {
    if (state.busy || state.recoveryWarnings.length) return;
    const value = event.target.value;
    if (value === "") state.lod2Overrides.delete(asset.folder);
    else {
      const set = state.scan.textureSets[Number(value)];
      if (!set) return;
      state.lod2Overrides.set(asset.folder, textureOverrideFor(asset, set));
    }
    await scanFolder(state.root);
  });
  const select = elements.assetDetail.querySelector("#main-texture-set");
  select?.addEventListener("change", async () => {
    if (state.busy || state.recoveryWarnings.length) return;
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
    if (state.busy || state.recoveryWarnings.length || !asset.mainTextureSet) return;
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
    assetName: asset.name,
    textureSetFolder: textureSet.folder,
    textureSetName: textureSet.name,
  };
}

function serializedTextureOverrides() {
  return [...state.textureOverrides.values()];
}

function serializedAssetSettingsOverrides() {
  return [...state.assetSettingsOverrides.values()];
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
  return relative || "Project root";
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
    wioWindow: "Opaque Window (_Wio)",
    lod1WioWindow: "LOD1 Opaque Window (_Wio)",
    lod2WioWindow: "LOD2 Opaque Window (_Wio)",
    wimWindow: "Opaque Window (_Wim)",
    lod1WimWindow: "LOD1 Opaque Window (_Wim)",
    lod2WimWindow: "LOD2 Opaque Window (_Wim)",
    wifWindow: "Frosted Window (_Wif)",
    lod1WifWindow: "LOD1 Frosted Window (_Wif)",
    lod2WifWindow: "LOD2 Frosted Window (_Wif)",
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

initializeDragDrop();
const startupRoot = localStorage.getItem(lastProjectKey);
if (startupRoot) scanFolder(startupRoot, true);
