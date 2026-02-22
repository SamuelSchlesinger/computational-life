# Divergences from Paper / Reference Implementation

This document tracks known differences between this codebase and the
[Computational Life paper](https://arxiv.org/abs/2406.19108) and its
[reference CUDA/Go implementation](https://github.com/paradigms-of-intelligence/cubff).

## Interaction ordering

**Paper (equations 3–4):** For each pair, a "catalyst" is chosen uniformly at
random to determine whether exec(AB) or exec(BA) is run.

**Reference implementation (cubff):** No explicit coin flip. The program reached
first in the shuffled iteration order always occupies the first half of the
concatenated tape. Over many epochs the shuffle provides both orderings, but
within a single epoch the initiator always goes first.

**This codebase:** An independent coin flip (`rng.gen::<bool>()` at
`surface.rs:842`) randomizes the ordering for each pair, independent of who was
the iterator. This is arguably closer to the paper's stated model but differs
from the reference implementation.

## Spatial topology

**Paper:** Primary experiments use a 0D "primordial soup" (well-mixed, any
program can pair with any other). 2D experiments use a flat 240×135 grid with
Chebyshev distance ≤ 2 neighborhood.

**Reference implementation:** Supports both 0D soup and 2D grid via
`--interaction_pattern`.

**This codebase:** Always spatial — programs live on triangle mesh faces
(icosphere, torus, flat grid, hamster tunnel, or arbitrary OBJ) with geodesic
neighborhoods. There is no well-mixed 0D mode.

## Features not implemented

- **Z80 / Intel 8080 emulation:** The paper explores self-replicators on real
  CPU instruction sets. Not implemented here.
- **Long tape simulations:** A single shared 65,536-byte tape with multiple
  execution threads. Not implemented here.
- **Tracer tokens:** 64-bit (epoch, position, char) tokens attached to every
  byte for lineage tracing. Not implemented here.

## Features added beyond the paper

- **Arbitrary mesh topologies** with geodesic neighborhoods (icosphere, torus,
  hamster tunnel, OBJ import).
- **Real-time 3D visualization** (Bevy + egui) with multiple color modes
  (hash, entropy, instruction density, neighbor similarity, territorial
  dominance).
- **Click-to-inspect program viewer** with disassembly.
- **Geometric-skip mutation sampling** (~240× speedup over naive per-byte
  iteration).
- **Deterministic seeded RNG** for full reproducibility.
