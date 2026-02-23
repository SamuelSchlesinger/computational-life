# Change: Reconcile specs with novel substrates and minor code divergences

## Why
Four novel substrates (Qop, Skim, Rig, Bits) were added to the codebase without corresponding specs, and existing specs for simulation-cli and live-visualization still reference only the original 4 substrates. A minor color mode discrepancy also exists (Zeros mode uses grayscale in code but spec says heatmap).

## What Changes
- Add new spec: `qop-interpreter` documenting the Queue-Operate-Produce substrate
- Add new spec: `skim-interpreter` documenting the Skip-chain machine substrate
- Add new spec: `rig-interpreter` documenting the Register-Indirect Goto substrate
- Add new spec: `bits-interpreter` documenting the Bit-serial machine substrate
- Modify `simulation-cli` to list all 8 substrates instead of only 4
- Modify `live-visualization` to reflect actual color mode display labels and the Zeros grayscale behavior

## Impact
- Affected specs: qop-interpreter (new), skim-interpreter (new), rig-interpreter (new), bits-interpreter (new), simulation-cli, live-visualization
- Affected code: none — this is a spec-only change to match existing code
