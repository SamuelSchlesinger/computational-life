## ADDED Requirements

### Requirement: Substrate Trait

The system SHALL define a `Substrate` trait that encapsulates instruction set execution semantics. The trait SHALL expose a single static method `execute(tape: &mut [u8], step_limit: usize) -> usize` that runs the program encoded in `tape` for at most `step_limit` steps and returns the number of steps actually executed. The tape SHALL be modified in-place during execution.

#### Scenario: Substrate executes within step limit
- **WHEN** a substrate's `execute` method is called with a tape and step_limit of 1000
- **THEN** it SHALL return a value less than or equal to 1000
- **AND** the tape MAY be modified

#### Scenario: Substrate handles empty program
- **WHEN** a substrate's `execute` method is called with a tape of all zeros
- **THEN** it SHALL terminate without panicking
- **AND** return the number of steps consumed (which may be 0)

### Requirement: Program Concatenation

The system SHALL support concatenating two programs of size `N` bytes into a single tape of `2*N` bytes for execution. After execution, the tape SHALL be split back into two programs of `N` bytes each. The first `N` bytes become the first program and the last `N` bytes become the second program.

#### Scenario: Two programs are concatenated and split
- **WHEN** program A (64 bytes) and program B (64 bytes) are concatenated
- **THEN** the resulting tape SHALL be 128 bytes with A occupying bytes 0..63 and B occupying bytes 64..127
- **AND** after execution, bytes 0..63 SHALL replace program A and bytes 64..127 SHALL replace program B
