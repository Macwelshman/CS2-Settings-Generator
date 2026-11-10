const { test } = require("node:test");
const assert = require("node:assert/strict");
const { encode, decode, partition, empty, findings } = require("../ui/project.js");

test("project configuration round-trips and can be rebased after a move", () => {
  for (const [root, newRoot, sep] of [["/work", "/moved", "/"], ["C:\\Work", "D:\\Moved", "\\"]]) {
    const choices = {
      projectMain: { textureSetFolder: root, textureSetName: "Brick" },
      textureOverrides: [{ assetFolder: root + sep + "House", assetName: "House",
        textureSetFolder: root + sep + "Sets", textureSetName: "Slate" }],
      lod2Overrides: [{ assetFolder: root + sep + "House", assetName: "House",
        textureSetFolder: root, textureSetName: "Atlas" }],
      assetSettingsOverrides: [{ assetFolder: root + sep + "Sign", assetName: "Sign",
        assetType: "decal", normalOpacity: 0.4 }],
    };
    const encoded = JSON.parse(JSON.stringify(encode(root, choices)));
    assert.equal(encoded.projectMain.textureSetFolder, ".");
    assert.equal(encoded.textureOverrides[0].textureSetFolder, "Sets");
    assert.deepEqual(decode(encoded, root).choices, choices);
    assert.equal(decode(encoded, newRoot).choices.lod2Overrides[0].assetFolder, newRoot + sep + "House");
  }
});

test("missing and renamed assets retain choices without inheriting them", () => {
  const choices = [{ assetFolder: "/work/House", assetName: "House" }];
  assert.deepEqual(partition(choices, []), []);
  assert.deepEqual(partition(choices, [{ folder: "/work/House", name: "Replacement" }]), []);
  assert.deepEqual(partition(choices, [{ folder: "/work/House", name: "House" }]), choices);
  assert.equal(choices.length, 1);
});

test("v1 migrates across Mac and Windows without the originating root", () => {
  const v1 = { ...empty(), version: 1, root: "/Volumes/Work",
    projectMain: { textureSetFolder: "Shared/Brick", textureSetName: "Brick" },
    lod2Overrides: [{ assetFolder: "Houses/A", assetName: "A",
      textureSetFolder: "Shared/LOD2", textureSetName: "Atlas" }],
    assetSettingsOverrides: [{ assetFolder: "Signs", assetName: "Sign", assetType: "decal", normalOpacity: 0 }] };
  const windows = "D:\\Moved";
  const migrated = decode(v1, windows).choices;
  assert.equal(migrated.projectMain.textureSetFolder, "D:\\Moved\\Shared\\Brick");
  assert.equal(migrated.lod2Overrides[0].assetFolder, "D:\\Moved\\Houses\\A");
  const exported = encode(windows, migrated);
  assert.equal(exported.version, 2);
  assert.equal(exported.root, undefined);
  assert.equal(decode(exported, "/Moved").choices.assetSettingsOverrides[0].normalOpacity, 0);
});

test("partial recovery keeps valid records and leaves the original profile intact", () => {
  const profile = { ...empty(), version: 2,
    textureOverrides: [{ assetFolder: "../Outside", assetName: "A", textureSetFolder: ".", textureSetName: "A" },
      { assetFolder: "B", assetName: "B", textureSetFolder: ".", textureSetName: "B" }],
    lod2Overrides: null };
  const original = JSON.stringify(profile);
  assert.throws(() => decode(profile, "/Work"), /relative path/);
  const { choices, warnings } = decode(profile, "/Work", true);
  assert.equal(warnings.length, 2);
  assert.equal(choices.textureOverrides[0].assetFolder, "/Work/B");
  assert.equal(JSON.stringify(profile), original);
  assert.throws(() => decode({ ...profile, version: 99 }, "/Work", true), /version/);
});

test("import detects an unavailable provider and a missing map in an otherwise available set", () => {
  const choices = { ...empty(), projectMain: { textureSetFolder: "/Work", textureSetName: "Brick" } };
  const set = { tier: "main", folder: "/Work", name: "Brick",
    files: [{ path: "/Work/Brick_BaseColor.png" }, { path: "/Work/Brick_Normal.png" }] };
  const profile = encode("/Work", choices, { assets: [], textureSets: [set] });
  const { choices: moved, expectedTextures } = decode(profile, "/Moved");
  const remaining = { ...set, folder: "/Moved", files: [{ path: "/Moved/Brick_BaseColor.png" }] };
  assert.deepEqual(findings(moved, { assets: [], textureSets: [remaining] }, { expectedTextures }),
    ["Expected texture is missing or could not be scanned: /Moved/Brick_Normal.png."]);
  assert.match(findings(moved, { assets: [], textureSets: [{ ...remaining, tier: "lod2" }] })[0], /Unavailable main texture provider/);
});

test("import detects LOD2 overrides incompatible with a decal and invalid opacity", () => {
  const selection = { assetFolder: "/Work/A", assetName: "A", textureSetFolder: "/Work", textureSetName: "Atlas" };
  const choices = { ...empty(), lod2Overrides: [selection] };
  const scan = { assets: [{ folder: "/Work/A", name: "A", assetType: "decal", files: [{ kind: "lod2" }], issues: [] }],
    textureSets: [{ folder: "/Work", name: "Atlas", tier: "lod2", files: [] }] };
  assert.match(findings(choices, scan)[0], /cannot use a LOD2 provider/);
  assert.throws(() => encode("/Work", { ...choices, assetSettingsOverrides: [
    { assetFolder: "/Work/A", assetName: "A", assetType: "decal", normalOpacity: 2 },
  ] }), /Normal opacity/);
});

function desktopSession(entries, invoke) {
  const vm = require("node:vm");
  const fs = require("node:fs");
  const storage = new Map(entries);
  const elements = new Map();
  function element(id) {
    if (!elements.has(id)) elements.set(id, {
      classList: { add() {}, remove() {} },
      addEventListener() {},
      querySelectorAll: () => [],
      querySelector: (selector) => element(id + selector),
      insertAdjacentHTML() {},
      remove() {},
    });
    return elements.get(id);
  }
  const context = vm.createContext({
    ProjectProfiles: require("../ui/project.js"),
    document: { querySelector: element },
    window: { __TAURI__: { core: { invoke } } },
    localStorage: { getItem: (key) => storage.get(key) ?? null, setItem: (key, value) => storage.set(key, value) },
    clearTimeout() {}, setTimeout() {},
  });
  vm.runInContext(fs.readFileSync(require.resolve("../ui/app.js"), "utf8"), context);
  return { element, storage, run: (source) => vm.runInContext(source, context) };
}

test("startup rescans the last project, applies surviving choices, and preserves a damaged save", async () => {
  const root = "/Work";
  const key = "cs2-settings:project:" + root;
  const saved = JSON.stringify({ ...empty(), version: 2, lod2Overrides: null,
    projectMain: { textureSetFolder: "Shared", textureSetName: "Brick" } });
  const scans = [];
  const session = desktopSession([[key, saved], ["cs2-settings:last-project", root]], async (cmd, args) => {
    assert.equal(cmd, "scan_export_folder");
    scans.push(args);
    return { root, assets: [], globalIssues: [], textureSets: [
      { folder: "/Work/Shared", name: "Brick", tier: "main", files: [] },
    ] };
  });
  await new Promise(setImmediate);
  assert.equal(scans.length, 2);
  assert.equal(scans[1].textureOptions.projectMain.textureSetFolder, "/Work/Shared");
  assert.equal(session.storage.get(key), saved);
  assert.equal(session.element("#generate").disabled, true);
  assert.equal(session.run("state.recoveryWarnings.length"), 1);
});

test("import validates before asking to replace a saved profile and respects a decline", async () => {
  const root = "/Work";
  const key = "cs2-settings:project:" + root;
  const saved = JSON.stringify({ ...empty(), version: 1, root });
  const commands = [];
  let approve = false;
  const session = desktopSession([[key, saved]], async (cmd) => {
    commands.push(cmd);
    if (cmd === "confirm_profile_replacement") return approve;
    return { root, assets: [], globalIssues: [], textureSets: [] };
  });
  await session.run(`scanFolder("/Work", true, '${JSON.stringify({ ...empty(), version: 99 })}', true)`);
  assert.deepEqual(commands, ["scan_export_folder"]);
  const valid = JSON.stringify({ ...empty(), version: 2 });
  await session.run(`scanFolder("/Work", true, '${valid}', true)`);
  assert.equal(commands.at(-1), "confirm_profile_replacement");
  assert.equal(session.storage.get(key), saved);
  assert.equal(session.storage.has("cs2-settings:last-project"), false);
  approve = true;
  await session.run(`scanFolder("/Work", true, '${valid}', true)`);
  assert.equal(JSON.parse(session.storage.get(key)).version, 2);
  assert.equal(session.storage.get("cs2-settings:last-project"), root);
});

test("an exported profile detects missing meshes even without per-asset overrides", () => {
  const profile = encode("/Work", empty(), { textureSets: [], assets: [
    { files: [{ path: "/Work/House.fbx" }] },
  ] });
  const decoded = decode(profile, "/Moved");
  assert.deepEqual(findings(decoded.choices, { assets: [], textureSets: [] }, decoded),
    ["Expected FBX is missing or could not be read: /Moved/House.fbx."]);
});
