## ADDED Requirements
### Requirement: Meta Update Rules

The Meta (Heterogeneous Cellular Automaton) substrate SHALL implement a parallel cellular automaton where every byte on the tape simultaneously acts as a local rule. There is no program counter. Each execution step updates ALL cells in parallel using double-buffering (read from current state, write to a buffer, then swap).

Each byte SHALL be interpreted as follows:
- Bits 7-6 (CONDITION): determines when the cell fires
  - `00`: ALWAYS — fire unconditionally
  - `01`: IF_LEFT_ZERO — fire only if left neighbor is 0x00
  - `10`: IF_RIGHT_ZERO — fire only if right neighbor is 0x00
  - `11`: IF_ANY_ZERO — fire if either neighbor is 0x00
- Bits 5-4 (ACTION): determines what happens when the cell fires
  - `00`: STAY — no change (cell keeps its current value)
  - `01`: COPY_RIGHT — write own value to right neighbor
  - `10`: COPY_LEFT — write own value to left neighbor
  - `11`: SWAP_RIGHT — exchange own value with right neighbor
- Bits 3-0 (PAYLOAD): data bits carried along with the cell; used for conflict resolution and identity

Neighbor addressing SHALL wrap modulo tape length (cell 0's left neighbor is the last cell; the last cell's right neighbor is cell 0).

Conflict resolution: when multiple cells attempt to write to the same target position in a single step, the write from the cell with the highest payload (bits 3-0) SHALL win. If payloads are equal, the leftmost cell (lowest index) SHALL win. A cell's own STAY action SHALL have lower priority than any incoming write.

A cell that does not fire SHALL retain its current value unless overwritten by a neighbor's action.

The substrate SHALL implement the `Substrate` trait. The `execute` function SHALL run `step_limit` global CA steps, returning the number of steps executed. Execution SHALL terminate early if the tape has not changed between two consecutive steps (fixed point reached). `is_instruction` SHALL return true for any byte where the ACTION bits (5-4) are non-zero (i.e., the cell does something when it fires). Execution on an empty tape (all zeros) SHALL return 0 steps immediately (no cells fire because ALWAYS+STAY is a no-op).

#### Scenario: 1-byte rightward replicator
- **WHEN** a tape has a single byte with CONDITION=IF_RIGHT_ZERO, ACTION=COPY_RIGHT (e.g., 0x91) at position 0 and all other bytes are 0x00
- **THEN** after one step position 1 SHALL contain the same byte (0x91)
- **AND** position 0 SHALL still contain 0x91
- **AND** the pattern SHALL continue spreading rightward on subsequent steps

#### Scenario: Fixed-point termination
- **WHEN** all cells have ACTION=STAY
- **THEN** the tape SHALL not change between steps
- **AND** execution SHALL terminate early

#### Scenario: Conflict resolution by payload
- **WHEN** cell A (payload 5) and cell B (payload 3) both attempt to COPY into the same target cell
- **THEN** cell A's value SHALL win because 5 > 3

#### Scenario: Bidirectional pattern
- **WHEN** a 2-byte pattern has one COPY_RIGHT cell and one COPY_LEFT cell
- **THEN** the pattern SHALL spread in both directions simultaneously

### Requirement: Meta Disassembly

The `disassemble` method SHALL produce one line per cell in the format `{addr:04X}: {byte:02X}  {condition} {action} payload={n}`. Condition SHALL be one of ALWAYS, IF_LEFT_ZERO, IF_RIGHT_ZERO, IF_ANY_ZERO. Action SHALL be one of STAY, COPY_RIGHT, COPY_LEFT, SWAP_RIGHT.

#### Scenario: Disassembly of replicator cell
- **WHEN** disassembling byte 0x91 at address 0x0000
- **THEN** the output SHALL show "IF_RIGHT_ZERO COPY_RIGHT payload=1"
