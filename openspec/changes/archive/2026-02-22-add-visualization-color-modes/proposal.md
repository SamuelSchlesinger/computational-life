# Change: Add alternative visualization color modes

## Why

The surface viewer currently colors every cell by hashing its program bytes, which only distinguishes "different program" from "same program." Richer color modes—hamming weight, byte entropy, zero-byte fraction, and per-epoch change delta—would let the user observe distinct evolutionary dynamics (structure formation, zero-poisoning, activity fronts) at a glance without leaving the live viewer.

## What Changes

- Add a color mode selector to the egui side panel (dropdown or radio buttons) allowing the user to switch modes at runtime without restarting.
- Implement five color-mapping functions in `viz.rs`, each converting a program (and optionally previous-epoch state) to an RGBA color:
  - **Hash** (current default): FNV hash of program bytes → RGB.
  - **Hamming weight**: popcount of all program bytes → grayscale or heatmap.
  - **Byte entropy**: Shannon entropy of the program's byte distribution → heatmap.
  - **Zero fraction**: proportion of zero bytes → dark-to-bright gradient.
  - **Change delta**: Hamming distance from previous epoch's program → bright = changed, dark = stable.
- Move the blur strength slider from `--blur` CLI flag into the GUI panel alongside the color mode selector, so both are runtime-adjustable.
- The `--blur` CLI flag remains as a way to set the initial value but the GUI control is authoritative at runtime.
- Track previous-epoch program snapshots on the sim thread for the `delta` mode.
- Send the selected color mode from the render thread to the sim thread via the existing command channel.

## Impact

- Affected specs: `live-visualization`
- Affected code: `src/viz.rs` (color functions, sim thread loop, UI panel, command channel), `src/main.rs` (minor: pass initial blur value)
