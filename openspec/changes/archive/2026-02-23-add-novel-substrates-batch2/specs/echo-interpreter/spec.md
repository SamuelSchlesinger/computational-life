## ADDED Requirements
### Requirement: Echo Instruction Set

The Echo (Delay-Line Memory) substrate SHALL implement a machine where the write address is always constrained to `read_pointer + delay`. The machine SHALL have a program counter (PC, usize, starting at 0), a read pointer (RP, usize, starting at 0), a delay register (DELAY, u8, initialized to `tape.len() / 2`), and an accumulator (ACC, u8, starting at 0). All pointer arithmetic SHALL wrap modulo tape length.

The write pointer (WP) SHALL always equal `(RP + DELAY) % tape.len()` and SHALL NOT be independently settable.

The following 16 opcodes (0x00-0x0F) SHALL be defined; bytes 0x10-0xFF SHALL be treated as no-ops:

| Opcode | Mnemonic | Behavior |
|--------|----------|----------|
| 0x00 | HALT | Terminate execution |
| 0x01 | ECHO | Copy tape[RP] to tape[WP]; advance RP by 1 |
| 0x02 | LOAD | ACC = tape[RP]; advance RP by 1 |
| 0x03 | STORE | tape[WP] = ACC (does not advance RP) |
| 0x04 | SKIP | Advance RP by 1 without copying |
| 0x05 | SET_DELAY | 2-byte: DELAY = tape[PC+1]; PC advances by 2 total |
| 0x06 | INC | ACC += 1 (wrapping) |
| 0x07 | DEC | ACC -= 1 (wrapping) |
| 0x08 | XOR | ACC ^= tape[RP] (does not advance RP) |
| 0x09 | ADD | ACC = ACC.wrapping_add(tape[RP]); advance RP |
| 0x0A | JMP_REL | 2-byte: relative jump by signed offset in tape[PC+1] |
| 0x0B | JZ | 2-byte: jump if ACC == 0, offset in tape[PC+1] |
| 0x0C | JNZ | 2-byte: jump if ACC != 0, offset in tape[PC+1] |
| 0x0D | SKIP_EQ | Skip next instruction if tape[RP] == tape[WP] |
| 0x0E | GET_DELAY | ACC = DELAY |
| 0x0F | SET_RP | RP = ACC |

Jump instructions SHALL compute the target as `PC + 2 + offset` where offset is the next byte interpreted as i8. A jump to a negative address SHALL terminate execution.

The substrate SHALL implement the `Substrate` trait. `is_instruction` SHALL return true for bytes 0x00-0x0F. Execution on an empty tape SHALL return 0 steps.

#### Scenario: 3-byte self-replicator
- **WHEN** a tape contains [0x01, 0x0A, 0xFD] (ECHO, JMP_REL, -3) in the first half
- **THEN** after execution the second half SHALL contain a copy of the first half
- **AND** the delay defaults to tape.len()/2 so ECHO copies to the correct offset

#### Scenario: ECHO copies one byte with delay offset
- **WHEN** RP is 5 and DELAY is 32 and an ECHO instruction executes
- **THEN** tape[37] SHALL receive the value of tape[5]
- **AND** RP SHALL advance to 6

#### Scenario: SET_DELAY changes write offset
- **WHEN** SET_DELAY executes with the next byte being 10
- **THEN** DELAY SHALL become 10
- **AND** all subsequent writes SHALL go to RP + 10

#### Scenario: Write pointer is not independently settable
- **WHEN** any sequence of instructions executes
- **THEN** the write position SHALL always equal (RP + DELAY) % tape.len()

### Requirement: Echo Disassembly

The `disassemble` method SHALL produce one line per instruction in the format `{addr:04X}: {byte:02X}  {mnemonic}`. For 2-byte instructions (SET_DELAY, JMP_REL, JZ, JNZ), the line SHALL additionally show the operand value.

#### Scenario: Disassembly of SET_DELAY instruction
- **WHEN** disassembling a SET_DELAY instruction at address 0x0000 with operand 64
- **THEN** the output SHALL include the mnemonic and the delay value
