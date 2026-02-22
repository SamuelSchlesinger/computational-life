## ADDED Requirements

### Requirement: High-Order Entropy Computation

The system SHALL compute the high-order entropy (HOE) metric as defined in the paper. HOE is computed by concatenating the entire population into a single byte buffer, compressing it with brotli at quality level 2, and dividing the compressed size by the raw (uncompressed) size. This approximates the normalized Kolmogorov complexity of the population.

#### Scenario: HOE of uniform random data
- **WHEN** HOE is computed on a freshly initialized random population
- **THEN** the value SHALL be close to 1.0 (near-incompressible)

#### Scenario: HOE of homogeneous population
- **WHEN** HOE is computed on a population where all programs are identical copies
- **THEN** the value SHALL be significantly less than 1.0 (highly compressible)

#### Scenario: HOE uses brotli quality 2
- **WHEN** the population is compressed for HOE computation
- **THEN** brotli compression SHALL be used at quality level 2, matching the paper's `brotli -q2`
