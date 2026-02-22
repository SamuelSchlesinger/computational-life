# subleq-interpreter Specification

## Purpose
TBD - created by archiving change reconcile-specs-with-codebase. Update Purpose after archive.
## Requirements
### Requirement: SUBLEQ Instruction Format and Execution

The system SHALL implement the standard SUBLEQ (SUbtract and Branch if Less than or EQual to zero) instruction set from paper Section 3.2. Each instruction consists of 3 consecutive bytes (a, b, c):

1. Compute: `tape[a] = tape[a] - tape[b]` (wrapping unsigned subtraction)
2. Compare: if `tape[a]` interpreted as signed (i8) is <= 0, set PC = tape[c] (read AFTER subtraction)
3. Otherwise: PC += 3

All addresses SHALL wrap modulo tape length. Execution SHALL terminate if PC + 2 >= tape length (insufficient bytes for a complete instruction). Every byte value is a valid instruction component (`is_instruction` returns true for all bytes).

#### Scenario: Branch on negative result
- **WHEN** tape[a] - tape[b] produces a value whose signed interpretation is negative
- **THEN** the program counter SHALL be set to tape[c]
- **AND** tape[c] SHALL be read after the subtraction (matters when a == c)

#### Scenario: No branch on positive result
- **WHEN** tape[a] - tape[b] produces a value whose signed interpretation is positive
- **THEN** the program counter SHALL advance by 3

#### Scenario: Termination on insufficient tape
- **WHEN** the program counter is at a position where PC + 2 >= tape length
- **THEN** execution SHALL terminate

### Requirement: RSUBLEQ4 Instruction Format and Execution

The system SHALL implement the RSUBLEQ4 (Relative SUBLEQ with 4 operands) instruction set. Each instruction consists of 4 consecutive bytes (a, b, c, d):

1. Compute addresses: addr_a = (PC + a) % len, addr_b = (PC + b) % len, addr_c = (PC + c) % len
2. Compute: `tape[addr_a] = tape[addr_b] - tape[addr_c]` (wrapping unsigned subtraction)
3. Compare: if `tape[addr_a]` interpreted as signed (i8) is <= 0, set PC = PC + d (where d is signed i8)
4. Otherwise: PC += 4

Execution SHALL terminate if PC + 3 >= tape length or if a backward branch would make PC negative. Every byte value is a valid instruction component.

The relative addressing scheme enables a 25-byte self-replicator (compared to 60 bytes for standard SUBLEQ), making spontaneous emergence more likely.

#### Scenario: Relative addressing
- **WHEN** an RSUBLEQ4 instruction at PC=10 has operands a=2, b=3, c=4
- **THEN** the addresses used SHALL be 12, 13, 14 (each offset from PC, wrapping modulo tape length)

#### Scenario: Signed branch offset
- **WHEN** operand d is 0xFE (signed value -2)
- **AND** the subtraction result is <= 0
- **THEN** the program counter SHALL move backward by 2

#### Scenario: Backward branch termination
- **WHEN** a backward branch would produce a negative program counter
- **THEN** execution SHALL terminate

