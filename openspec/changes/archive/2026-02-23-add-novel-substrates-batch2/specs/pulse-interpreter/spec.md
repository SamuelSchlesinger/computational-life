## ADDED Requirements
### Requirement: Pulse Instruction Set

The Pulse (Signal/Collision) substrate SHALL implement a parallel particle machine with no program counter. Each byte on the tape SHALL represent either an empty cell (0x00) or a particle. Particle encoding: bit 7 = direction (0 = rightward, 1 = leftward), bits 6-0 = particle type (1-127, where 0 means empty).

Each execution step SHALL proceed in two phases:

1. **Movement phase**: All particles simultaneously advance one position in their direction (rightward or leftward), wrapping modulo tape length. Particle positions are updated into a staging buffer to avoid order-dependent artifacts.

2. **Collision phase**: When two or more particles occupy the same cell after movement, they interact according to fixed collision rules:
   - Two particles of the same type and opposite directions SHALL annihilate (both become 0x00)
   - Two particles of different types SHALL produce two new particles: the output types SHALL be computed as `(type_a + type_b) % 127 + 1` and `(type_a ^ type_b) % 127 + 1`, with the first product moving rightward and the second moving leftward
   - When more than two particles collide, they SHALL be resolved pairwise in order of ascending type

The substrate SHALL implement the `Substrate` trait. The `execute` function SHALL run `step_limit` global movement-and-collision steps, returning the number of steps executed. `is_instruction` SHALL return true for any non-zero byte. Execution on an empty tape (all zeros) SHALL return 0 steps. Execution SHALL terminate early if all particles have annihilated.

#### Scenario: Single particle moves
- **WHEN** a tape has a single rightward particle (type 1, direction 0) at position 5
- **THEN** after one step the particle SHALL be at position 6
- **AND** position 5 SHALL be empty (0x00)

#### Scenario: Collision produces new particles
- **WHEN** a rightward particle of type 3 and a leftward particle of type 5 occupy the same cell
- **THEN** two new particles SHALL be produced according to the collision rules
- **AND** one SHALL move rightward and one SHALL move leftward

#### Scenario: Same-type annihilation
- **WHEN** two particles of the same type but opposite directions collide
- **THEN** both SHALL be removed (cell becomes 0x00)

#### Scenario: Empty tape
- **WHEN** the tape contains only 0x00 bytes
- **THEN** execute SHALL return 0 steps

### Requirement: Pulse Disassembly

The `disassemble` method SHALL produce one line per tape position in the format `{addr:04X}: {byte:02X}  {description}`. Empty cells (0x00) SHALL be labeled `EMPTY`. Particles SHALL show direction (LEFT/RIGHT) and type number.

#### Scenario: Disassembly of particle
- **WHEN** disassembling byte 0x83 (leftward, type 3) at address 0x0005
- **THEN** the output SHALL include "LEFT type=3"
