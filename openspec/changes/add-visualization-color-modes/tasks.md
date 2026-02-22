## 1. Color mode infrastructure

- [ ] 1.1 Define `ColorMode` enum (`Hash`, `Hamming`, `Entropy`, `Zeros`, `Delta`) and add it to `SimCommand` so the render thread can send mode changes to the sim thread
- [ ] 1.2 Add `ColorMode` state to the sim thread loop; use it to select which color fill function runs before sending snapshots

## 2. Color mapping functions

- [ ] 2.1 Implement `fill_colors_hamming` — map total popcount of program bytes to a heatmap gradient
- [ ] 2.2 Implement `fill_colors_entropy` — map per-program Shannon entropy to a heatmap gradient
- [ ] 2.3 Implement `fill_colors_zeros` — map zero-byte fraction to a dark-to-bright gradient
- [ ] 2.4 Implement `fill_colors_delta` — map Hamming distance from previous-epoch snapshot to a bright (changed) / dark (stable) gradient; store previous-epoch programs buffer on the sim thread

## 3. GUI controls

- [ ] 3.1 Add a color mode dropdown/radio group to the egui side panel (surface view)
- [ ] 3.2 Add a blur strength slider to the egui side panel, replacing reliance on the `--blur` CLI flag for runtime control
- [ ] 3.3 Send `SimCommand` messages when the user changes color mode or blur strength

## 4. Validation

- [ ] 4.1 Verify all five modes render correctly on a small surface (e.g., `--surface sphere:3 --live`)
- [ ] 4.2 Verify mode switching is responsive (no restart required, updates within one snapshot interval)
- [ ] 4.3 Verify blur slider works at runtime and initial `--blur` CLI value is respected as the starting position
