# Project Status

## Project

CS2 Settings

## Current state

The desktop generator now saves per-project selections, restores or relocates
the last project, and compares existing settings with the proposed JSON.
FBX scanning checks exporter naming, mesh material slots and LOD relationships
in Import Readiness.

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

Manual desktop smoke testing on macOS and Windows remains useful.

## Next recommended action

Use the architecture map in AGENTS.md to inspect only the components relevant to the next requested change.

## Known issues

None recorded here yet.
