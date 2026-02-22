## ADDED Requirements

### Requirement: BFF Instruction Set

The system SHALL implement the BFF (Brainfuck Family) instruction set as defined in Section 2 of the paper. The interpreter operates on a byte tape with three pointers: an instruction pointer (IP, starting at 0), a read head (head0, starting at 0), and a write head (head1, starting at 0). All pointers are stored as single bytes representing positions on the tape. All arithmetic on tape values and head positions SHALL wrap modulo 256. The following instructions SHALL be implemented:

| Byte | Char | Operation |
|------|------|-----------|
| 0x3C | `<`  | head0 = head0 - 1 (wrapping) |
| 0x3E | `>`  | head0 = head0 + 1 (wrapping) |
| 0x7B | `{`  | head1 = head1 - 1 (wrapping) |
| 0x7D | `}`  | head1 = head1 + 1 (wrapping) |
| 0x2D | `-`  | tape[head0] = tape[head0] - 1 (wrapping) |
| 0x2B | `+`  | tape[head0] = tape[head0] + 1 (wrapping) |
| 0x2E | `.`  | tape[head1] = tape[head0] |
| 0x2C | `,`  | tape[head0] = tape[head1] |
| 0x5B | `[`  | if tape[head0] == 0: jump IP forward to matching `]` |
| 0x5D | `]`  | if tape[head0] != 0: jump IP backward to matching `[` |

All other byte values (246 out of 256) SHALL be treated as no-ops. The IP advances by 1 after each instruction (except when a bracket causes a jump).

#### Scenario: Head movement wraps
- **WHEN** head0 is 0 and a `<` instruction is executed
- **THEN** head0 SHALL become 255
- **AND WHEN** head0 is 255 and a `>` instruction is executed
- **THEN** head0 SHALL become 0

#### Scenario: Arithmetic wraps
- **WHEN** tape[head0] is 0 and a `-` instruction is executed
- **THEN** tape[head0] SHALL become 255

#### Scenario: Copy operations
- **WHEN** a `.` instruction is executed with head0=5 and head1=10
- **THEN** tape[10] SHALL be set to the value of tape[5]
- **AND WHEN** a `,` instruction is executed with head0=5 and head1=10
- **THEN** tape[5] SHALL be set to the value of tape[10]

#### Scenario: No-op bytes
- **WHEN** the IP points to a byte that is not one of the 10 defined instructions
- **THEN** the interpreter SHALL treat it as a no-op and advance IP by 1

### Requirement: BFF Bracket Matching

Bracket matching in BFF SHALL follow standard nesting rules. `[` and `]` form matched pairs with arbitrary nesting depth. If a `[` has no matching `]` (or vice versa), the program SHALL terminate immediately upon encountering the unmatched bracket. Bracket matching SHALL be computed by scanning the instruction bytes of the tape.

#### Scenario: Matched brackets with zero value
- **WHEN** the IP is at a `[` and tape[head0] == 0
- **THEN** the IP SHALL jump forward to the instruction immediately after the matching `]`

#### Scenario: Matched brackets with non-zero value
- **WHEN** the IP is at a `]` and tape[head0] != 0
- **THEN** the IP SHALL jump backward to the matching `[`

#### Scenario: Unmatched bracket terminates
- **WHEN** the IP is at a `[` with no matching `]` in the tape
- **THEN** the program SHALL terminate immediately

### Requirement: BFF Step Limit

The BFF interpreter SHALL enforce a configurable step limit. Each instruction executed (including no-ops) counts as one step. When the step limit is reached, execution SHALL stop immediately. The default step limit SHALL be 2^13 (8192), matching the paper.

#### Scenario: Execution stops at step limit
- **WHEN** a program is executed with a step limit of 8192
- **AND** the program has not terminated naturally by step 8192
- **THEN** execution SHALL stop and the tape state at that point SHALL be the final state

### Requirement: BFF Implements Substrate Trait

The BFF interpreter SHALL implement the `Substrate` trait so that it can be used interchangeably with other instruction sets in the simulation engine.

#### Scenario: BFF used as substrate
- **WHEN** the simulation engine is configured with the BFF substrate
- **THEN** it SHALL use the BFF interpreter for all program executions within the primordial soup loop
