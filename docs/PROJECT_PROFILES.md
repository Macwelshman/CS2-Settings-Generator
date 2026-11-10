# Project profiles and session recovery

**Export Project Profile…** writes a JSON file from the open project. Copy the
profile and the export folder separately, keeping the internal folder layout.
**Import Project Profile…** asks for the profile and the destination export
folder. The app rescans that folder and checks the profile against what it finds
before applying any choices. If the destination already has saved choices, the
app asks before replacing them. Export requires an unused filename. Neither
action writes any asset's `settings.json`.

## Format

The desktop app uses the same profile codec for local saves and exports. Version
2 omits the source computer's absolute root. Paths use forward slashes and are
relative to the chosen export folder; `.` represents the root itself. Absolute
paths and parent traversal are invalid. Names and paths must match the scanner's
asset and texture-set names at the destination.

```json
{
  "version": 2,
  "projectMain": { "textureSetFolder": "Shared", "textureSetName": "Brick" },
  "textureOverrides": [],
  "lod2Overrides": [
    { "assetFolder": "Houses/A", "assetName": "A", "textureSetFolder": "Houses", "textureSetName": "Atlas" }
  ],
  "assetSettingsOverrides": [
    { "assetFolder": "Signs", "assetName": "Sign", "assetType": "decal", "normalOpacity": 0.4 }
  ]
}
```

A null `projectMain` means automatic detection. An entry in `textureOverrides`
uses the same fields as a LOD2 override and takes priority over the project
provider for main and LOD1 textures. Without a LOD2 override, the existing
scanner inheritance rules apply to the destination tree. Asset settings accept
`standard` or `decal`; optional decal normal opacity ranges from 0 to 1.

Exports include an optional `expectedTextures` list of project-relative PNG
paths from selected and resolved sets. It lets import detect a missing map even
if other maps in that set remain. The optional `expectedFbx` list checks for
missing or unreadable meshes, including assets without overrides. A missing or renamed asset referenced by an
override, a missing provider, or a provider of the wrong tier also rejects
import. The profile does not embed FBX or PNG files. Version 1 local profiles
are accepted: their old absolute `root` field is ignored and their relative
paths are resolved at the chosen destination. Exporting or saving a change
writes version 2. Unsupported versions are never guessed.

## Recovery

On launch, including after an in-place update, the app rescans the last
successfully opened project using local desktop storage. It applies saved
selections and rebuilds Import Readiness and the Changes Only preview from the
current files. Replacement of existing asset settings is off after reopening.
If the folder has moved, use **Locate Moved Project…** to rebase the saved
choices; this does not replace a different profile already saved for the
destination.

Missing or renamed assets and unavailable providers are reported during
recovery. Their saved choices are retained. If only part of a saved profile is
valid, the app shows a preview with the valid entries and reports the entries
it could not read. It preserves the original saved data and disables generation,
editing and profile export for that recovery view. Import a corrected profile
and approve replacing the saved choices to return to normal use. If the JSON or
version cannot be read at all, the app shows the base scan with the same
protections. Export a profile before moving to another machine or clearing app
data; the last-project pointer and local saves belong to this installation.
