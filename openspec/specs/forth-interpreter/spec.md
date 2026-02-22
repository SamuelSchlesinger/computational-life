# forth-interpreter Specification

## Purpose
TBD - created by archiving change reconcile-specs-with-codebase. Update Purpose after archive.
## Requirements
### Requirement: Forth Instruction Set Encoding

The system SHALL implement a Forth-family instruction set (from paper Section 3.1.1) where each byte decodes as follows:

- Bits `0000_xxxx` (0x00-0x0D): fixed opcodes (14 operations, see Forth Fixed Opcodes)
- Bits `0000_xxxx` (0x0E-0x3F): no-op
- Bits `01xx_xxxx` (0x40-0x7F): push-immediate; the low 6 bits (0-63) SHALL be pushed onto the stack
- Bits `1x_xxxxxx` (0x80-0xFF): relative jump; bit 6 is the sign (0=forward, 1=backward), low 6 bits + 1 is the offset

All bytes not matching a defined operation SHALL be treated as no-ops.

#### Scenario: Push-immediate decoding
- **WHEN** the instruction pointer encounters byte 0x45 (binary 01_000101)
- **THEN** the value 5 SHALL be pushed onto the stack

#### Scenario: Forward jump decoding
- **WHEN** the instruction pointer encounters byte 0x82 (binary 10_000010)
- **THEN** the program counter SHALL advance forward by 3 (low 6 bits = 2, offset = 2+1)

#### Scenario: Backward jump decoding
- **WHEN** the instruction pointer encounters byte 0xC3 (binary 11_000011)
- **THEN** the program counter SHALL move backward by 4 (low 6 bits = 3, offset = 3+1)
- **AND** if the offset exceeds the current PC, execution SHALL terminate

### Requirement: Forth Fixed Opcodes

The Forth substrate SHALL implement 14 fixed opcodes (low 4 bits, when high bits are 0000):

| Opcode | Hex  | Mnemonic | Operation |
|--------|------|----------|-----------|
| 0000   | 0x00 | READ     | Push tape[top] (pop address, push value) |
| 0001   | 0x01 | READ64   | Push tape[top + 64] |
| 0010   | 0x02 | WRITE    | tape[top] = second; pop both |
| 0011   | 0x03 | WRITE64  | tape[top + 64] = second; pop both |
| 0100   | 0x04 | DUP      | Duplicate top of stack |
| 0101   | 0x05 | POP      | Discard top of stack |
| 0110   | 0x06 | SWAP     | Swap top two stack elements |
| 0111   | 0x07 | SKIPNZ   | If top != 0, skip next instruction (pc++) |
| 1000   | 0x08 | INC      | Increment top of stack by 1 |
| 1001   | 0x09 | DEC      | Decrement top of stack by 1 |
| 1010   | 0x0A | ADD      | second = top + second; pop |
| 1011   | 0x0B | SUB      | second = top - second; pop |
| 1100   | 0x0C | COPY     | tape[top + 64] = tape[top]; pop |
| 1101   | 0x0D | RCOPY    | tape[top] = tape[top + 64]; pop |

All tape addresses SHALL wrap modulo tape length. All byte arithmetic SHALL wrap modulo 256.

#### Scenario: COPY enables trivial self-replicator
- **WHEN** a single COPY instruction (0x0C) executes on an empty stack
- **THEN** it SHALL pop 0 (underflow), copy tape[0] to tape[64]
- **AND** this enables spontaneous self-replication in the primordial soup

#### Scenario: Arithmetic wrapping
- **WHEN** INC is applied to a top-of-stack value of 255
- **THEN** the result SHALL be 0 (wrapping modulo 256)

### Requirement: Forth Stack Behavior

The Forth substrate SHALL use a fixed-size stack of 256 bytes (MAX_STACK = 256).

- Stack underflow SHALL return 0 (not panic or terminate). This is critical for allowing self-replicators to emerge from random programs.
- Stack overflow (push when full) SHALL silently drop the value.
- All stack operations SHALL be safe regardless of stack state.

#### Scenario: Underflow returns zero
- **WHEN** POP or any consuming operation is performed on an empty stack
- **THEN** the consumed value SHALL be 0
- **AND** execution SHALL continue normally

#### Scenario: Overflow drops silently
- **WHEN** a push is attempted with the stack at capacity (256 elements)
- **THEN** the value SHALL be discarded
- **AND** execution SHALL continue normally

