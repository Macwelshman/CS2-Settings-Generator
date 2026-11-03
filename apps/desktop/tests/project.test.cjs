const { test } = require("node:test");
const assert = require("node:assert/strict");
const { encode, decode, partition } = require("../ui/project.js");

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
    assert.deepEqual(decode(encoded, root), { ...choices, root, version: 1 });
    assert.equal(decode(encoded, newRoot).lod2Overrides[0].assetFolder, newRoot + sep + "House");
  }
});

test("missing and renamed assets retain choices without inheriting them", () => {
  const choices = [{ assetFolder: "/work/House", assetName: "House" }];
  assert.deepEqual(partition(choices, []), []);
  assert.deepEqual(partition(choices, [{ folder: "/work/House", name: "Replacement" }]), []);
  assert.deepEqual(partition(choices, [{ folder: "/work/House", name: "House" }]), choices);
  assert.equal(choices.length, 1);
});
