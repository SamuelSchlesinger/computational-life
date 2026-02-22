# Computational Life

This is an independent reproduction of the paper
[**Computational Life: How Well-formed, Self-replicating Programs Emerge from Simple Interaction**](https://arxiv.org/abs/2406.19108)
by Blaise Ag​üera y Arcas, Jyrki Alakuijala, James Evans, Ben Laurie,
Alexander Mordvintsev, Eyvind Niklasson, Ettore Randazzo, and Luca Versari
(Google, Paradigms of Intelligence Team & The University of Chicago).

All credit for the ideas, experimental design, and instruction sets goes to the
original authors. This project exists purely as an exercise in scientific
curiosity — we found their work fascinating and wanted to watch self-replicators
emerge with our own eyes.

<p align="center">
  <img src="images/surface-sphere.png" width="720" alt="Programs evolving on an icosphere surface" />
</p>

Programs are random byte strings that execute against each other. Two programs
are concatenated, run through an instruction set interpreter, and the modified
tape is split back — allowing programs to read, overwrite, and replicate into
their neighbors. Over thousands of epochs, replicators emerge, compete for
space, and drive down the entropy of the population.

## Gallery

| 2D grid — early spatial clustering | 2D grid — late stage diversity |
|:---:|:---:|
| ![2D early](images/2d-early-spatial.png) | ![2D late](images/2d-late-stage.png) |

| Surface simulation on a torus |
|:---:|
| ![Torus](images/surface-torus.png) |

## Features

- **Two instruction sets**: BFF (Brainfuck-family with dual read/write heads)
  and Forth (stack-based with push literals and relative jumps)
- **Multiple topologies**: 0D (well-mixed), 2D flat grid, and arbitrary 3D
  surfaces (icosphere, torus, flat mesh, `.obj` import)
- **Live visualization** via Bevy with real-time metrics (high-order entropy,
  unique program count, zero-byte count)
- **Deterministic** — seeded RNG for full reproducibility
- **Fast** — geometric-skip mutation, parallel surface epochs via Rayon

## Quick start

```bash
# Headless run (CSV to stdout)
cargo run --release -- --seed 42 --epochs 10000

# 2D spatial simulation with live viewer
cargo run --release --features viz -- --seed 42 --epochs 100000 \
  --surface flat:240x135 --live

# 3D surface on an icosphere
cargo run --release --features viz -- --seed 42 --epochs 10000 \
  --surface sphere:5 --live

# Torus surface
cargo run --release --features viz -- --seed 42 --epochs 10000 \
  --surface torus:80x80 --live

# Benchmark throughput
cargo run --release -- --seed 1 --epochs 1000 --benchmark
```

## CLI options

| Flag | Default | Description |
|------|---------|-------------|
| `--seed` | *required* | RNG seed for reproducibility |
| `--epochs` | *required* | Number of epochs to simulate |
| `--population-size` | 131072 | Program count (0D mode only) |
| `--program-size` | 64 | Bytes per program |
| `--step-limit` | 8192 | Max interpreter steps per interaction |
| `--mutation-rate` | 0.00024 | Per-byte mutation probability per epoch |
| `--substrate` | `bff` | Instruction set (`bff` or `forth`) |
| `--surface` | *none* | Surface spec: `flat:WxH`, `sphere:N`, `torus:MxN`, `obj:PATH` |
| `--live` | off | Launch Bevy visualization window (requires `--features viz`) |
| `--blur` | 0.0 | Spatial blur strength for the live viewer |
| `--benchmark` | off | Print throughput stats instead of CSV |

## How it works

1. **Initialize** a population of random byte-string programs
2. **Each epoch**: every program interacts with a neighbor — two programs are
   concatenated, executed by the chosen instruction set, and the result is
   split back into the two slots
3. **Mutate**: random bit-flips at a low per-byte rate (geometric skip for
   efficiency)
4. **Measure**: high-order entropy (brotli compression ratio) tracks whether
   structure is emerging in the population

When replicators arise, they copy themselves into neighbors, collapsing
entropy and driving up the count of identical programs. Different instruction
sets and topologies produce qualitatively different evolutionary dynamics.

## References

> Blaise Ag​üera y Arcas, Jyrki Alakuijala, James Evans, Ben Laurie,
> Alexander Mordvintsev, Eyvind Niklasson, Ettore Randazzo, Luca Versari.
> *Computational Life: How Well-formed, Self-replicating Programs Emerge
> from Simple Interaction.* arXiv:2406.19108, 2024.
> https://arxiv.org/abs/2406.19108

## License

MIT
