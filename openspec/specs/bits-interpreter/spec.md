# bits-interpreter Specification

## Purpose
TBD - created by archiving change reconcile-novel-substrates. Update Purpose after archive.
## Requirements
### Requirement: Bits Instruction Set

The Bits (Bit-serial) substrate SHALL implement a machine that operates on individual bits, providing 8x finer granularity than byte-oriented substrates. The machine SHALL have a byte-addressed program counter (PC, usize, starting at 0), a bit-addressed read pointer (BP, usize, starting at 0), a bit-addressed write pointer (WP, usize, starting at `tape.len() * 8 / 2`), and a 1-bit carry register (u8, starting at 0).

Bit addressing SHALL use the formula: byte index = `pos / 8`, bit index = `pos % 8` (LSB-first within each byte). All bit positions SHALL wrap modulo `tape.len() * 8`.

The following 15 opcodes (0x0–0xE) SHALL be defined; opcode 0xF SHALL be a no-op:

| Opcode | Mnemonic | Behavior |
|--------|----------|----------|
| 0x0 | COPY_BIT | Write bit at BP to WP; advance both |
| 0x1 | SET_BIT | Write 1 to WP; advance WP |
| 0x2 | CLR_BIT | Write 0 to WP; advance WP |
| 0x3 | SKIP_BIT | Advance BP by 1 |
| 0x4 | READ_CARRY | Load bit at BP into carry; advance BP |
| 0x5 | WRITE_CARRY | Write carry to WP; advance WP |
| 0x6 | FLIP_CARRY | Toggle carry (carry ^= 1) |
| 0x7 | AND_CARRY | carry &= bit at BP; advance BP |
| 0x8 | OR_CARRY | carry |= bit at BP; advance BP |
| 0x9 | XOR_CARRY | carry ^= bit at BP; advance BP |
| 0xA | JZ_CARRY | 2-byte: if carry == 0, relative jump by signed offset in tape[pc+1] |
| 0xB | JNZ_CARRY | 2-byte: if carry != 0, relative jump by signed offset in tape[pc+1] |
| 0xC | BP_RESET | Reset BP to 0 |
| 0xD | WP_RESET | Reset WP to tape.len() * 8 / 2 |
| 0xE | HALT | Terminate execution |

Jump instructions SHALL compute the target as `pc + 2 + offset` where offset is the next byte interpreted as i8. A jump to a negative address SHALL terminate execution. The substrate SHALL implement the `Substrate` trait. `is_instruction` SHALL return true when the high nibble is <= 0xE. Execution on an empty tape SHALL return 0 steps.

#### Scenario: 4-byte self-replicator
- **WHEN** a tape begins with [0x00, 0x60, 0xB0, 0xFC] (COPY_BIT, FLIP_CARRY, JNZ_CARRY, offset -4)
- **THEN** after execution all bits from the first half SHALL be copied to the second half

#### Scenario: Bit-level granularity
- **WHEN** COPY_BIT executes with BP=3 and WP=515
- **THEN** bit 3 of byte 0 SHALL be written to bit 3 of byte 64
- **AND** BP SHALL advance to 4 and WP to 516

#### Scenario: Carry as boolean logic
- **WHEN** carry is 1 and AND_CARRY executes with the bit at BP being 0
- **THEN** carry SHALL become 0

### Requirement: Bits Disassembly

The `disassemble` method SHALL produce one line per instruction in the format `{addr:04X}: {byte:02X}  {mnemonic}`. For 2-byte jump instructions, the line SHALL additionally show the signed offset and computed target address.

#### Scenario: Disassembly of conditional jump
- **WHEN** disassembling a JNZ_CARRY instruction at address 0x0002 with offset -4
- **THEN** the output SHALL include the mnemonic, signed offset, and target address

