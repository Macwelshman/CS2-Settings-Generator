// The same configuration is used for local saves and exported profiles.
const ProjectProfiles = (() => {
  const keys = ["textureOverrides", "lod2Overrides", "assetSettingsOverrides"];
  const empty = () => ({ projectMain: null, ...Object.fromEntries(keys.map((key) => [key, []])) });
  const object = (value) => value && typeof value === "object" && !Array.isArray(value);
  const name = (value) => typeof value === "string" && value.trim().length > 0;

  const separatorFor = (root) => root.startsWith("/") ? "/" : "\\";

  function relative(root, path) {
    const separator = separatorFor(root);
    const base = root.endsWith(separator) ? root.slice(0, -1) : root;
    if (path === root) return ".";
    if (path.startsWith(base + separator)) {
      const child = path.slice(base.length + 1);
      return separator === "\\" ? child.replaceAll("\\", "/") : child;
    }
    throw new Error("A saved selection is outside the project folder.");
  }

  function resolve(root, path) {
    if (path === ".") return root;
    if (typeof path !== "string") throw new Error("Expected a project-relative path.");
    const parts = path.split("/");
    if (parts.some((part) => !part || part === "." || part === ".." || /[\\:\x00-\x1f]/.test(part))) {
      throw new Error("Invalid project-relative path.");
    }
    const separator = separatorFor(root);
    return (root.endsWith(separator) ? root : root + separator) + parts.join(separator);
  }

  function read(profile, path, recover) {
    if (!object(profile) || ![1, 2].includes(profile.version)) {
      throw new Error("Unsupported project configuration version.");
    }
    const choices = empty();
    const warnings = [];
    function attempt(label, action) {
      try { action(); } catch (error) {
        const message = `${label}: ${error.message}`;
        if (!recover) throw new Error(message);
        warnings.push(message);
      }
    }
    attempt("Profile fields", () => {
      const allowed = ["version", "projectMain", ...keys,
        ...(profile.version === 1 ? ["root"] : ["expectedTextures", "expectedFbx"])];
      if (Object.keys(profile).some((key) => !allowed.includes(key))) {
        throw new Error("Unsupported configuration fields.");
      }
    });
    const selection = (item, key) => {
      if (!object(item)) throw new Error("Expected a selection.");
      const fields = [
        ...(key === "projectMain" ? [] : ["assetName", "assetFolder"]),
        ...(key === "assetSettingsOverrides" ? ["assetType", "normalOpacity"] : ["textureSetName", "textureSetFolder"]),
      ];
      if (Object.keys(item).some((field) => !fields.includes(field))) {
        throw new Error("Unsupported selection fields.");
      }
      const result = { ...item };
      if (key !== "projectMain") {
        if (!name(item.assetName)) throw new Error("Missing asset name.");
        result.assetFolder = path(item.assetFolder);
      }
      if (key === "assetSettingsOverrides") {
        if (!["standard", "decal"].includes(item.assetType)) throw new Error("Unsupported asset type.");
        if (item.normalOpacity != null && (item.assetType !== "decal" ||
          !Number.isFinite(item.normalOpacity) || item.normalOpacity < 0 || item.normalOpacity > 1)) {
          throw new Error("Normal opacity must be between 0 and 1 and is only supported for decals.");
        }
      } else {
        if (!name(item.textureSetName)) throw new Error("Missing texture set name.");
        result.textureSetFolder = path(item.textureSetFolder);
      }
      return result;
    };
    attempt("Project texture provider", () => {
      if (profile.projectMain !== null) choices.projectMain = selection(profile.projectMain, "projectMain");
    });
    for (const key of keys) {
      attempt(key, () => {
        if (!Array.isArray(profile[key])) throw new Error("Expected a list of selections.");
        const folders = new Set();
        profile[key].forEach((item, index) => attempt(`${key} entry ${index + 1}`, () => {
          const value = selection(item, key);
          if (folders.has(value.assetFolder)) throw new Error("Duplicate asset folder.");
          folders.add(value.assetFolder);
          choices[key].push(value);
        }));
      });
    }
    const expected = {};
    for (const key of ["expectedTextures", "expectedFbx"]) {
      expected[key] = [];
      if (profile.version === 2 && Object.hasOwn(profile, key)) {
        attempt(key, () => {
          if (!Array.isArray(profile[key])) throw new Error("Expected a list of paths.");
          expected[key] = profile[key].map((value) => {
            if (value === ".") throw new Error("Expected a file path.");
            return path(value);
          });
        });
      }
    }
    return { choices, warnings, ...expected };
  }

  function encode(root, choices, scan) {
    const { choices: relativeChoices } = read({ ...choices, version: 2 }, (path) => {
      const value = relative(root, path);
      resolve(root, value);
      return value;
    }, false);
    const profile = { version: 2, ...relativeChoices };
    if (scan) {
      const sets = scan.assets.flatMap((asset) => [asset.mainTextureSet, asset.lod2TextureSet]).filter(Boolean);
      const selected = [choices.projectMain, ...choices.textureOverrides, ...choices.lod2Overrides].filter(Boolean);
      sets.push(...scan.textureSets.filter((set) => selected.some((item) =>
        item.textureSetFolder === set.folder && item.textureSetName === set.name)));
      profile.expectedTextures = [...new Set(sets.flatMap((set) => set.files.map((file) => relative(root, file.path))))].sort();
      profile.expectedFbx = [...new Set(scan.assets.flatMap((asset) =>
        asset.files.map((file) => relative(root, file.path))))].sort();
      decode(profile, root);
    }
    return profile;
  }

  function decode(profile, root, recover = false) {
    return read(profile, (path) => resolve(root, path), recover);
  }

  function partition(choices, assets) {
    const known = new Map(assets.map((asset) => [asset.folder, asset.name]));
    return choices.filter((choice) => known.get(choice.assetFolder) === choice.assetName);
  }

  function findings(choices, scan, { expectedTextures = [], expectedFbx = [] } = {}) {
    const knownTextures = new Set(scan.textureSets.flatMap((set) => set.files.map((file) => file.path)));
    const issues = expectedTextures.filter((path) => !knownTextures.has(path))
      .map((path) => `Expected texture is missing or could not be scanned: ${path}.`);
    const knownFbx = new Set(scan.assets.flatMap((asset) => asset.files
      .filter((file) => !file.parseError).map((file) => file.path)));
    for (const path of expectedFbx) {
      if (!knownFbx.has(path)) issues.push(`Expected FBX is missing or could not be read: ${path}.`);
    }
    function provider(item, tier) {
      if (item && !scan.textureSets.some((set) => set.tier === tier &&
        set.folder === item.textureSetFolder && set.name === item.textureSetName)) {
        issues.push(`Unavailable ${tier} texture provider “${item.textureSetName}” in ${item.textureSetFolder}.`);
      }
    }
    provider(choices.projectMain, "main");
    for (const item of choices.textureOverrides) provider(item, "main");
    for (const item of choices.lod2Overrides) provider(item, "lod2");
    for (const key of keys) {
      for (const item of choices[key]) {
        const asset = scan.assets.find((asset) => asset.folder === item.assetFolder && asset.name === item.assetName);
        if (!asset) issues.push(`Asset “${item.assetName}” was not found in ${item.assetFolder}.`);
        else if (key === "lod2Overrides" && (asset.assetType === "decal" || !asset.files.some((file) => file.kind === "lod2"))) {
          issues.push(`“${item.assetName}” cannot use a LOD2 provider with its current meshes and asset type.`);
        }
      }
    }
    for (const asset of scan.assets) {
      for (const issue of asset.issues) {
        if (issue.code === "decalRequiredTextureMissing" || issue.code === "decalTextureSetUnresolved") {
          issues.push(`${asset.name}: ${issue.message}`);
        }
      }
    }
    return [...new Set(issues)];
  }

  return { encode, decode, partition, empty, findings };
})();

if (typeof module !== "undefined") module.exports = ProjectProfiles;
