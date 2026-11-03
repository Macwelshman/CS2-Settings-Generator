// Versioned desktop project profiles. Paths inside a project use slash separators.
const ProjectProfiles = (() => {
  function relative(root, path) {
    const base = root.replace(/[\\/]+$/, "");
    if (path === root) return ".";
    if (path.startsWith(base + "/") || path.startsWith(base + "\\")) {
      return path.slice(base.length + 1).replaceAll("\\", "/");
    }
    throw new Error("A saved selection is outside the project folder.");
  }

  function resolve(root, path) {
    if (path === ".") return root;
    const parts = path.split("/");
    if (parts.some((part) => !part || part === "." || part === ".." || /[\\:]/.test(part))) {
      throw new Error("Invalid relative path in project configuration.");
    }
    const separator = root.includes("\\") ? "\\" : "/";
    return root.replace(/[\\/]+$/, "") + separator + parts.join(separator);
  }

  function transform(profile, path) {
    const selection = (item) => ({
      ...item,
      ...(item.assetFolder !== undefined ? { assetFolder: path(item.assetFolder) } : {}),
      ...(item.textureSetFolder !== undefined ? { textureSetFolder: path(item.textureSetFolder) } : {}),
    });
    return {
      ...profile,
      projectMain: profile.projectMain ? selection(profile.projectMain) : null,
      textureOverrides: profile.textureOverrides.map(selection),
      lod2Overrides: profile.lod2Overrides.map(selection),
      assetSettingsOverrides: profile.assetSettingsOverrides.map(selection),
    };
  }

  function encode(root, choices) {
    return transform({ ...choices, version: 1, root }, (path) => relative(root, path));
  }

  function decode(profile, root) {
    if (profile.version !== 1) throw new Error("Unsupported project configuration version.");
    return transform(profile, (path) => resolve(root, path));
  }

  function partition(choices, assets) {
    const known = new Map(assets.map((asset) => [asset.folder, asset.name]));
    return choices.filter((choice) => known.get(choice.assetFolder) === choice.assetName);
  }

  return { encode, decode, partition };
})();

if (typeof module !== "undefined") module.exports = ProjectProfiles;
