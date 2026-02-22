## ADDED Requirements

### Requirement: Forth Instruction Set Encoding

The system SHALL implement the Forth primordial soup instruction set from Section 3.1.1 of the paper. Each instruction is one byte, decoded as follows:
- Bits 7-6 = `00`, bits 3-0 select one of 14 fixed opcodes (values 0x00-0x0D). Byte values 0x0E-0x3F are no-ops.
- Bits 7-6 = `01`: push the low 6 bits as an unsigned value onto the stack.
- Bit 7 = `1`: relative jump by `±([low 6 bits] + 1)`, sign determined by bit 6 (0 = forward, 1 = backward).

All other bit patterns SHALL be treated as no-ops.

#### Scenario: Fixed opcode decoding
- **WHEN** a byte with bits 7-6 = `00` and bits 3-0 in range 0x00-0x0D is encountered
- **THEN** the corresponding fixed opcode SHALL be executed

#### Scenario: Push immediate
- **WHEN** a byte with bits 7-6 = `01` is encountered (e.g., 0x43 = push 3)
- **THEN** the low 6 bits SHALL be pushed onto the stack as an unsigned value

#### Scenario: Relative jump
- **WHEN** a byte with bit 7 = `1` is encountered
- **THEN** the PC SHALL jump by `±([low 6 bits] + 1)` relative to the current position
- **AND** the sign SHALL be determined by bit 6 (0 = forward/positive, 1 = backward/negative)

#### Scenario: No-op bytes
- **WHEN** a byte in the range 0x0E-0x3F is encountered
- **THEN** it SHALL be treated as a no-op and the PC SHALL advance by 1

### Requirement: Forth Fixed Opcodes

The system SHALL implement the following 14 fixed opcodes (byte values 0x00-0x0D), where `<top>` is the top of the stack, `<top-1>` is the second element, and `*addr` denotes tape memory at address `addr % tape_len`:

| Byte | Mnemonic | Operation |
|------|----------|-----------|
| 0x00 | READ     | `<top> = *<top>` |
| 0x01 | READ64   | `<top> = *(<top> + 64)` |
| 0x02 | WRITE    | `*<top> = <top-1>; pop; pop` |
| 0x03 | WRITE64  | `*(<top> + 64) = <top-1>; pop; pop` |
| 0x04 | DUP      | `push <top>` (duplicate top) |
| 0x05 | POP      | `pop` (discard top) |
| 0x06 | SWAP     | swap `<top>` and `<top-1>` |
| 0x07 | SKIPNZ   | if `<top> != 0`: skip next instruction (`pc += 2` instead of `pc += 1`) |
| 0x08 | INC      | `<top> = <top> + 1` |
| 0x09 | DEC      | `<top> = <top> - 1` |
| 0x0A | ADD      | `<top-1> = <top> + <top-1>; pop` |
| 0x0B | SUB      | `<top-1> = <top> - <top-1>; pop` |
| 0x0C | COPY     | `*(<top> + 64) = *<top>; pop` |
| 0x0D | RCOPY    | `*<top> = *(<top> + 64); pop` |

All arithmetic SHALL wrap modulo 256 (u8). All tape addresses SHALL wrap modulo the tape length.

#### Scenario: READ loads from tape
- **WHEN** the stack top is 10 and tape[10] contains 42
- **THEN** after READ, the stack top SHALL be 42

#### Scenario: WRITE stores to tape
- **WHEN** the stack contains [value=99, addr=10] (top is addr)
- **THEN** after WRITE, tape[10] SHALL be 99 and both values SHALL be popped

#### Scenario: COPY between tape halves
- **WHEN** the stack top is 5 and tape[5] contains 0xAB
- **THEN** after COPY, tape[5 + 64] SHALL contain 0xAB and the address SHALL be popped

#### Scenario: DUP duplicates top
- **WHEN** the stack contains [3]
- **THEN** after DUP, the stack SHALL contain [3, 3]

#### Scenario: Arithmetic wraps
- **WHEN** the stack top is 255
- **THEN** after INC, the stack top SHALL be 0

### Requirement: Forth Stack Behavior

The Forth interpreter SHALL use a bounded stack. Stack underflow (popping from an empty stack or accessing elements that don't exist) SHALL terminate execution immediately. Stack overflow SHALL also terminate execution. Stack values SHALL be unsigned bytes (u8).

#### Scenario: Stack underflow terminates
- **WHEN** a POP instruction is executed with an empty stack
- **THEN** execution SHALL terminate immediately

#### Scenario: Stack overflow terminates
- **WHEN** the stack reaches its maximum capacity and a push is attempted
- **THEN** execution SHALL terminate immediately

### Requirement: Forth Substrate Trait Implementation

The Forth interpreter SHALL implement the `Substrate` trait with `execute(tape: &mut [u8], step_limit: usize) -> usize`. The PC SHALL start at 0 and the stack SHALL start empty. Execution SHALL terminate when: the PC moves out of bounds, the step limit is reached, or a stack error occurs.

#### Scenario: Forth used in primordial soup
- **WHEN** `--substrate forth` is specified
- **THEN** the simulation SHALL use the Forth interpreter for all program executions

#### Scenario: Step limit enforced
- **WHEN** a Forth program loops indefinitely
- **THEN** execution SHALL stop after `step_limit` steps and return `step_limit`

#### Scenario: Trivial self-replicator
- **WHEN** byte 0x0C (COPY) is executed on an empty-ish stack starting at position 0
- **THEN** it SHALL demonstrate the trivial one-byte self-replicator behavior described in the paper (copying itself to position 64)
