# qop-interpreter Specification

## Purpose
TBD - created by archiving change reconcile-novel-substrates. Update Purpose after archive.
## Requirements
### Requirement: Qop Instruction Set

The Qop (Queue-Operate-Produce) substrate SHALL implement a queue-based machine with an instruction pointer (PC), an accumulator (acc, u8), a read pointer (head, u8), and a write pointer (tail, u8). The head SHALL start at 0 and the tail SHALL start at `tape.len() / 2`. All pointer arithmetic SHALL wrap using u8 wrapping semantics, with tape access using modulo tape length.

The following 16 opcodes (0x00–0x0F) SHALL be defined; bytes 0x10–0xFF SHALL be treated as no-ops:

| Opcode | Mnemonic | Behavior |
|--------|----------|----------|
| 0x00 | HALT | Terminate execution |
| 0x01 | PASS | Copy tape[head] to tape[tail]; advance both pointers |
| 0x02 | EAT | Load tape[head] into acc; advance head |
| 0x03 | SPIT | Store acc to tape[tail]; advance tail |
| 0x04 | SKIP | Advance head by 1 |
| 0x05 | GAP | Write 0 to tape[tail]; advance tail |
| 0x06 | INC | Increment acc (wrapping) |
| 0x07 | DEC | Decrement acc (wrapping) |
| 0x08 | XOR | XOR acc with tape[head] (does not advance head) |
| 0x09 | JMP_REL | 2-byte: relative jump by signed offset in tape[pc+1] |
| 0x0A | JZ | 2-byte: jump if acc == 0 |
| 0x0B | JNZ | 2-byte: jump if acc != 0 |
| 0x0C | SET_HEAD | Set head to acc |
| 0x0D | SET_TAIL | Set tail to acc |
| 0x0E | GET_HEAD | Load head into acc |
| 0x0F | GET_TAIL | Load tail into acc |

Jump instructions (JMP_REL, JZ, JNZ) SHALL compute the target as `pc + 2 + offset` where offset is the next byte interpreted as i8. A jump to a negative address SHALL terminate execution.

The substrate SHALL implement the `Substrate` trait. `is_instruction` SHALL return true for bytes 0x00–0x0F. Execution on an empty tape SHALL return 0 steps.

#### Scenario: PASS copies byte through queue
- **WHEN** head points to a byte with value 42 and a PASS instruction executes
- **THEN** tape[tail] SHALL be set to 42
- **AND** both head and tail SHALL advance by 1

#### Scenario: 3-byte self-replicator
- **WHEN** a tape contains [0x01, 0x09, 0xFD] (PASS, JMP_REL, -3) in the first half
- **THEN** after execution the second half SHALL contain a copy of the first half

#### Scenario: NOP bytes are harmless
- **WHEN** a tape contains only bytes >= 0x10
- **THEN** execution SHALL complete without modifying any tape contents (other than PC advancement)

### Requirement: Qop Disassembly

The `disassemble` method SHALL produce one line per instruction in the format `{addr:04X}: {byte:02X}  {mnemonic}`. For 2-byte jump instructions, the line SHALL additionally show the signed offset and computed target address.

#### Scenario: Disassembly of jump instruction
- **WHEN** disassembling a JMP_REL instruction at address 0x0000 with offset -3
- **THEN** the output SHALL include the mnemonic, signed offset, and target address

