# Project Status

## Project

CS2 Settings

## Current state

The desktop generator saves per-project selections, rescans the last project on
startup, and imports and exports versioned portable profiles. Profile import
validates assets and texture providers before applying choices; partial recovery
preserves the original saved configuration. See [Project Profiles](docs/PROJECT_PROFILES.md).
Changes Only previews and FBX Import Readiness remain available.

Project documentation has been added to support efficient Codex development.

## Technical architecture

Recorded in [AGENTS.md](AGENTS.md#technical-architecture): source layout, entry points, UI/data flow, platforms, dependencies, builds, tests, and packaging.

## Current focus

Establish an efficient, low-context Codex development workflow while preserving existing application behaviour.

## Important priorities

- reliable CS2 integration
- safe settings handling
- preservation of user configuration
- clear UI
- predictable filesystem behaviour
- efficient development and testing

## Current task

The macOS smoke test passed profile export, import after moving the project,
both overwrite responses, and recovery after a normal restart and after the
updater helper replaced and relaunched an isolated bundle. The test used a
local staged update; release download and Windows desktop testing remain open.

## Next recommended action

Use the architecture map in AGENTS.md to inspect only the components relevant to the next requested change.

## Known issues

None recorded here yet.
