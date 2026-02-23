# rig-interpreter Specification

## Purpose
TBD - created by archiving change reconcile-novel-substrates. Update Purpose after archive.
## Requirements
### Requirement: Rig Instruction Set

The Rig (Register-Indirect Goto) substrate SHALL implement a Von Neumann register machine with a program counter (PC, usize) and 4 general-purpose registers (r0–r3, each u8). Register r0 SHALL start at 0, r1 SHALL start at `tape.len() / 2`, and r2/r3 SHALL start at 0. All register arithmetic SHALL wrap modulo 256. All tape addresses SHALL wrap modulo tape length.

Each instruction byte SHALL encode: high 4 bits as opcode, bits 3–2 as destination register index, bits 1–0 as source register index. The following 12 opcodes (0x0–0xB) SHALL be defined; opcodes 0xC–0xF SHALL be no-ops:

| Opcode | Mnemonic | Behavior |
|--------|----------|----------|
| 0x0 | LOAD | r[dst] = tape[r[src] % len] |
| 0x1 | STORE | tape[r[dst] % len] = r[src] |
| 0x2 | MOV | r[dst] = r[src] |
| 0x3 | ADD | r[dst] += r[src] (wrapping) |
| 0x4 | SUB | r[dst] -= r[src] (wrapping) |
| 0x5 | XOR | r[dst] ^= r[src] |
| 0x6 | INC | r[dst] += 1 (src ignored, wrapping) |
| 0x7 | DEC | r[dst] -= 1 (src ignored, wrapping) |
| 0x8 | JZ | If r[src] == 0: PC = r[dst] as usize |
| 0x9 | JNZ | If r[src] != 0: PC = r[dst] as usize |
| 0xA | COPY | tape[r[dst] % len] = tape[r[src] % len] |
| 0xB | HALT | Terminate execution |

Jump targets are absolute (the raw register value). If the target is >= tape length, execution SHALL terminate on the next iteration. The substrate SHALL implement the `Substrate` trait. `is_instruction` SHALL return true when the high nibble is <= 0xB. Execution on an empty tape SHALL return 0 steps.

#### Scenario: 4-byte self-replicator
- **WHEN** a tape begins with [0xA4, 0x60, 0x64, 0x9C] (COPY [r1],[r0]; INC r0; INC r1; JNZ r3,r0)
- **THEN** after execution the second half SHALL contain a copy of the first half

#### Scenario: Register-indirect copy
- **WHEN** a COPY instruction executes with r[dst]=10 and r[src]=5
- **THEN** tape[10] SHALL be set to the value of tape[5]

#### Scenario: NOP opcodes are harmless
- **WHEN** a byte has opcode nibble 0xC–0xF
- **THEN** execution SHALL advance PC by 1 without modifying any register or tape content

### Requirement: Rig Disassembly

The `disassemble` method SHALL produce one line per instruction in the format `{addr:04X}: {byte:02X}  {mnemonic}`, with register operands shown as r0–r3. Memory-indirect operands SHALL be shown with bracket notation (e.g., `COPY [r1], [r0]`).

#### Scenario: Disassembly shows register names
- **WHEN** disassembling byte 0xA4 (COPY, dst=r1, src=r0)
- **THEN** the output SHALL show `COPY [r1], [r0]`

