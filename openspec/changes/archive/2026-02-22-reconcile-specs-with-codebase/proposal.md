# Change: Reconcile specifications with current codebase

## Why

The specifications have fallen significantly behind the codebase. Four implemented changes (Forth interpreter, surface geometry, color modes, viewer optimization) were never archived, the application evolved from a CLI tool to a GUI-only application, and several features (SUBLEQ interpreters, program viewer, hamster tunnel) were added without specs. The specs describe a system that no longer exists.

## What Changes

- **ADD** `forth-interpreter`: Document the Forth substrate (14 fixed opcodes, push-immediate, relative jump)
- **ADD** `subleq-interpreter`: Document both Subleq and Rsubleq4 substrates
- **ADD** `surface-topology`: Document surface mesh system (icosphere, torus, flat grid, hamster tunnel, OBJ import, geodesic neighbors)
- **ADD** `program-viewer`: Document click-to-inspect program disassembly feature
- **MODIFY** `program-execution`: Add `is_instruction` and `disassemble` methods to Substrate trait
- **MODIFY** `soup-simulation`: Replace flat-array soup with surface-based spatial simulation
- **MODIFY** `live-visualization`: Rewrite to document GUI-only operation, 3D rendering, color modes, orbit camera, sidebar controls
- **REMOVE** `simulation-cli`: CLI no longer exists; all configuration is via GUI menu
- **REMOVE** `live-visualization` / Benchmark Mode: not implemented
- **REMOVE** `live-visualization` / Byte Frequency Histogram Plot: computed but not displayed

## Impact

- Affected specs: all 7 existing capabilities modified or removed, plus 4 new capabilities
- Affected code: none — this is documentation-only, bringing specs in line with existing code
- Prerequisite: archive 4 completed-but-unarchived changes with `--skip-specs`
