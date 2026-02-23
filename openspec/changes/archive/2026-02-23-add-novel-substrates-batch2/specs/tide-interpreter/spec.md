## ADDED Requirements
### Requirement: Tide Instruction Set

The Tide (Bidirectional Oscillating PC) substrate SHALL implement a machine where the program counter oscillates forward and backward, interpreting different nibbles of each byte depending on direction. The machine SHALL have a program counter (PC, usize, starting at 0), a direction flag (FORWARD/BACKWARD, starting as FORWARD), a bounce distance register (BOUNCE, u8, initialized to `tape.len() / 2`), a step-within-sweep counter (SWEEP_POS, u8, starting at 0), an accumulator (ACC, u8, starting at 0), a read pointer (RP, usize, starting at 0), and a write pointer (WP, usize, starting at `tape.len() / 2`).

On each execution step:
1. Read the byte at PC
2. If direction is FORWARD, decode the high nibble (bits 7-4) as the instruction
3. If direction is BACKWARD, decode the low nibble (bits 3-0) as the instruction
4. Execute the instruction
5. Advance PC by 1 in the current direction (increment if FORWARD, decrement if BACKWARD), wrapping modulo tape length
6. Increment SWEEP_POS. If SWEEP_POS reaches BOUNCE, set SWEEP_POS to 0 and reverse direction

Forward instructions (high nibble, 0x0-0xF):

| Opcode | Mnemonic | Behavior |
|--------|----------|----------|
| 0x0 | F_NOP | No operation |
| 0x1 | F_LOAD | ACC = tape[RP] |
| 0x2 | F_INC_RP | RP += 1 (wrapping) |
| 0x3 | F_DEC_RP | RP -= 1 (wrapping) |
| 0x4 | F_CMP | ACC = (tape[RP] == tape[WP]) as u8; 1 if equal, 0 if not |
| 0x5 | F_ADD | ACC = ACC.wrapping_add(tape[RP]) |
| 0x6 | F_XOR | ACC ^= tape[RP] |
| 0x7 | F_INC | ACC += 1 (wrapping) |
| 0x8 | F_DEC | ACC -= 1 (wrapping) |
| 0x9 | F_SET_BOUNCE | BOUNCE = ACC |
| 0xA | F_SKIP_Z | If ACC == 0, skip the next forward step (advance PC without executing) |
| 0xB | F_SKIP_NZ | If ACC != 0, skip the next forward step |
| 0xC | F_GET_RP | ACC = RP as u8 |
| 0xD | F_GET_WP | ACC = WP as u8 |
| 0xE | F_ECHO | Copy tape[RP] to tape[WP] (no pointer advancement) |
| 0xF | F_HALT | Terminate execution |

Backward instructions (low nibble, 0x0-0xF):

| Opcode | Mnemonic | Behavior |
|--------|----------|----------|
| 0x0 | B_NOP | No operation |
| 0x1 | B_STORE | tape[WP] = ACC |
| 0x2 | B_INC_WP | WP += 1 (wrapping) |
| 0x3 | B_DEC_WP | WP -= 1 (wrapping) |
| 0x4 | B_SWAP | Swap ACC with tape[WP] |
| 0x5 | B_ADD | ACC = ACC.wrapping_add(tape[WP]) |
| 0x6 | B_XOR | ACC ^= tape[WP] |
| 0x7 | B_INC | ACC += 1 (wrapping) |
| 0x8 | B_DEC | ACC -= 1 (wrapping) |
| 0x9 | B_SET_BOUNCE | BOUNCE = ACC |
| 0xA | B_SKIP_Z | If ACC == 0, skip the next backward step |
| 0xB | B_SKIP_NZ | If ACC != 0, skip the next backward step |
| 0xC | B_SET_RP | RP = ACC |
| 0xD | B_SET_WP | WP = ACC |
| 0xE | B_ECHO | Copy tape[RP] to tape[WP]; advance both RP and WP |
| 0xF | B_HALT | Terminate execution |

All pointer arithmetic SHALL wrap modulo tape length. The substrate SHALL implement the `Substrate` trait. `is_instruction` SHALL return true for all bytes (both nibbles are always meaningful). Execution on an empty tape SHALL return 0 steps.

#### Scenario: 3-byte self-replicator
- **WHEN** a tape contains bytes where:
  - Byte 0: high=F_LOAD(0x1), low=B_INC_WP(0x2) → 0x12
  - Byte 1: high=F_INC_RP(0x2), low=B_STORE(0x1) → 0x21
  - Byte 2: high=F_NOP(0x0), low=B_NOP(0x0) → 0x00
- **AND** BOUNCE is set to 3
- **THEN** the forward sweep executes F_LOAD, F_INC_RP, F_NOP (reads one byte)
- **AND** the backward sweep executes B_NOP, B_STORE, B_INC_WP (writes one byte and advances WP)
- **AND** after tape.len()/2 full oscillations, the second half SHALL contain a copy of the first half

#### Scenario: Direction reversal at bounce boundary
- **WHEN** SWEEP_POS reaches BOUNCE during a forward sweep
- **THEN** direction SHALL reverse to BACKWARD
- **AND** SWEEP_POS SHALL reset to 0

#### Scenario: Forward reads high nibble, backward reads low nibble
- **WHEN** the PC is at a byte with value 0x12
- **AND** direction is FORWARD
- **THEN** the instruction SHALL be F_LOAD (0x1)
- **WHEN** direction is BACKWARD at the same byte
- **THEN** the instruction SHALL be B_INC_WP (0x2)

### Requirement: Tide Disassembly

The `disassemble` method SHALL produce one line per byte in the format `{addr:04X}: {byte:02X}  fwd={forward_mnemonic} bwd={backward_mnemonic}`. Both the forward instruction (high nibble) and backward instruction (low nibble) SHALL be shown for each byte.

#### Scenario: Disassembly of dual-instruction byte
- **WHEN** disassembling byte 0x12 at address 0x0000
- **THEN** the output SHALL show "fwd=F_LOAD bwd=B_INC_WP"
