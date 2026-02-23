## ADDED Requirements

### Requirement: Skim Instruction Set

The Skim (Skip-chain) substrate SHALL implement a machine where every byte simultaneously encodes an opcode in its high nibble and a skip distance in its low nibble. The machine SHALL have a program counter (PC, usize), an accumulator (acc, u8, starting at 0), and a write pointer (wp, u8, starting at `tape.len() / 2`).

After each instruction, the next PC SHALL be computed as `(pc + (low_nibble & 0x0F) + 1) % tape.len()`, except when overridden by conditional skip instructions.

The high nibble SHALL select from the following 13 operations (0x0–0xC); nibbles 0xD–0xF SHALL be no-ops that still perform the skip:

| High nibble | Mnemonic | Behavior |
|-------------|----------|----------|
| 0x0 | LOAD | acc = tape[wp] |
| 0x1 | STORE | tape[wp] = acc |
| 0x2 | COPY_FWD | tape[wp] = tape[pc]; advance wp |
| 0x3 | INC | Increment acc (wrapping) |
| 0x4 | DEC | Decrement acc (wrapping) |
| 0x5 | XOR | acc ^= tape[wp] |
| 0x6 | WP_INC | Advance wp by 1 (wrapping) |
| 0x7 | WP_DEC | Retreat wp by 1 (wrapping) |
| 0x8 | SET_WP | wp = acc |
| 0x9 | GET_WP | acc = wp |
| 0xA | SKZ | If acc != 0: override skip to pc+1; else use normal skip |
| 0xB | SKNZ | If acc == 0: override skip to pc+1; else use normal skip |
| 0xC | HALT | Terminate execution |

The wp pointer SHALL use u8 wrapping arithmetic, with tape access modulo tape length. The substrate SHALL implement the `Substrate` trait. `is_instruction` SHALL return true when the high nibble is <= 0xC.

#### Scenario: Skip-chain advances PC by low nibble
- **WHEN** the byte at PC has low nibble 0x3
- **THEN** the next PC SHALL be (pc + 4) % tape.len()

#### Scenario: COPY_FWD enables self-replication
- **WHEN** a 64-byte tape is filled with 0x20 bytes (COPY_FWD, skip 1)
- **THEN** after execution the second half SHALL be a copy of the first half

#### Scenario: Data is control flow
- **WHEN** a byte in the data region is reached by the skip chain
- **THEN** it SHALL be interpreted as an instruction and its low nibble SHALL determine the next PC

### Requirement: Skim Disassembly

The `disassemble` method SHALL produce one line per byte in the format `{addr:04X}: {byte:02X}  {mnemonic:<10} skip {distance} -> {target:04X}`, showing the operation name, skip distance, and computed target address.

#### Scenario: Disassembly shows skip targets
- **WHEN** disassembling a byte 0x25 (COPY_FWD, skip 6) at address 0x0010
- **THEN** the output line SHALL show skip distance 6 and target address 0x0016
