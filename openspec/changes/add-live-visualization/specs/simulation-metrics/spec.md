## ADDED Requirements

### Requirement: Unique Program Count

The system SHALL compute the number of distinct programs in the population. Two programs are considered identical if all their bytes match. This metric corresponds to the "unique tokens" line in Figure 1 of the paper.

#### Scenario: All unique programs
- **WHEN** the population is freshly initialized with random data
- **THEN** the unique program count SHALL be close to the population size

#### Scenario: Dominated by replicators
- **WHEN** the population contains many copies of the same program
- **THEN** the unique program count SHALL be significantly less than the population size

### Requirement: Zero Byte Count

The system SHALL compute the total number of zero-valued bytes across the entire population. This metric tracks the "zero-poisoning" phenomenon described in the paper (Section 2.1, Figure 2).

#### Scenario: Random initialization
- **WHEN** the population is freshly initialized with uniform random bytes
- **THEN** the zero byte count SHALL be approximately population_size * program_size / 256

#### Scenario: Zero-poisoning phase
- **WHEN** self-replicators that produce zeros have emerged
- **THEN** the zero byte count SHALL increase significantly above the random baseline

### Requirement: Byte Frequency Histogram

The system SHALL compute a histogram of byte value frequencies across the entire population. The histogram SHALL have 256 bins (one per possible byte value) and each bin SHALL contain the count of occurrences of that byte value.

#### Scenario: Random population
- **WHEN** computed on a freshly initialized random population
- **THEN** all 256 bins SHALL have approximately equal counts

#### Scenario: Replicator-dominated population
- **WHEN** the population is dominated by programs using specific instruction bytes
- **THEN** the histogram SHALL show peaks at those byte values
