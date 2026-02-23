## ADDED Requirements
### Requirement: Worm Instruction Set

The Worm (Motile Self-Modifying Program) substrate SHALL implement a machine where the program physically moves through the tape. The machine SHALL maintain a head pointer (usize), a tail pointer (usize), a direction flag (forward/backward), and an accumulator (u8). The head SHALL start at 0, the tail SHALL start at 0, and direction SHALL start as forward (increasing addresses).

Each execution step:
1. Read the byte at the head position as an instruction
2. Execute the instruction
3. Advance the head by one position in the current direction (wrapping modulo tape length)
4. Unless the instruction is GROW, advance the tail by one position in the current direction, zeroing the vacated cell

The worm's body is the contiguous region between tail and head. If the worm's length reaches 0 (head equals tail), execution SHALL terminate.

The following 16 opcodes SHALL be defined using the high nibble (bits 7-4). The low nibble (bits 3-0) SHALL serve as an immediate operand:

| Opcode | Mnemonic | Behavior |
|--------|----------|----------|
| 0x0 | HALT | Terminate execution |
| 0x1 | NOP | No operation |
| 0x2 | EMIT | Write low nibble value to the cell ahead of head |
| 0x3 | COPY_SELF | Write the current instruction byte to the cell ahead of head |
| 0x4 | COPY_BODY | Write byte at position (tail + low_nibble) % tape_len to ahead of head |
| 0x5 | GROW | Delay tail advancement this step (worm grows by 1) |
| 0x6 | SHRINK | Advance tail by 2 instead of 1 (worm shrinks by 1), zeroing both vacated cells |
| 0x7 | TURN | Reverse direction |
| 0x8 | LOAD | acc = tape at position (head + low_nibble) % tape_len |
| 0x9 | STORE | Write acc to cell ahead of head |
| 0xA | INC | Increment acc (wrapping) |
| 0xB | DEC | Decrement acc (wrapping) |
| 0xC | XOR | acc ^= low_nibble |
| 0xD | SKIP_Z | Skip next instruction (advance head without executing) if acc == 0 |
| 0xE | SKIP_NZ | Skip next instruction if acc != 0 |
| 0xF | PEEK | acc = tape at cell ahead of head (non-destructive read ahead) |

The substrate SHALL implement the `Substrate` trait. `is_instruction` SHALL return true for all bytes (every byte is meaningful when the worm visits it). Execution on an empty tape SHALL return 0 steps.

#### Scenario: Single-byte COPY_SELF replicator
- **WHEN** a tape has a single COPY_SELF byte (0x30) at position 0 and all other bytes are zero
- **THEN** after execution the COPY_SELF byte SHALL propagate forward, leaving copies in its wake

#### Scenario: Worm erases behind itself
- **WHEN** the worm advances from position 5 to position 6
- **THEN** position 5 SHALL be set to 0x00 (unless GROW was executed)

#### Scenario: GROW extends worm body
- **WHEN** a GROW instruction executes
- **THEN** the tail SHALL NOT advance this step
- **AND** the worm's body length SHALL increase by 1

#### Scenario: Worm dies when length reaches zero
- **WHEN** SHRINK causes the tail to equal the head
- **THEN** execution SHALL terminate

### Requirement: Worm Disassembly

The `disassemble` method SHALL produce one line per byte in the format `{addr:04X}: {byte:02X}  {mnemonic} {operand}`. The operand SHALL show the low nibble value where applicable.

#### Scenario: Disassembly of COPY_BODY instruction
- **WHEN** disassembling byte 0x43 at address 0x0002
- **THEN** the output SHALL show "COPY_BODY 3" indicating body offset 3
