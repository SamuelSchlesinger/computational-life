/// Compute the high-order entropy (HOE) of a byte slice.
///
/// HOE = compressed_size / raw_size, where compression uses brotli at quality 2.
/// This approximates the normalized Kolmogorov complexity of the data.
///
/// Returns a value typically between 0 and 1, where:
/// - ~1.0 means the data is incompressible (random)
/// - <<1.0 means the data is highly structured/repetitive
///
/// Values slightly above 1.0 are possible due to compression overhead on random data.
pub fn high_order_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    let mut compressed = Vec::new();
    let params = brotli::enc::BrotliEncoderParams {
        quality: 2,
        ..Default::default()
    };
    brotli::BrotliCompress(&mut &data[..], &mut compressed, &params)
        .expect("brotli compression should not fail on valid input");

    compressed.len() as f64 / data.len() as f64
}

/// Count the number of distinct programs in the population.
pub fn unique_program_count(programs: &[Vec<u8>]) -> usize {
    use std::collections::HashSet;
    let set: HashSet<&[u8]> = programs.iter().map(|p| p.as_slice()).collect();
    set.len()
}

/// Count the total number of zero-valued bytes across all programs.
pub fn zero_byte_count(programs: &[Vec<u8>]) -> usize {
    programs
        .iter()
        .flat_map(|p| p.iter())
        .filter(|&&b| b == 0)
        .count()
}

/// Compute a histogram of byte value frequencies across all programs.
/// Returns an array of 256 counts, one per possible byte value.
pub fn byte_frequency_histogram(programs: &[Vec<u8>]) -> [usize; 256] {
    let mut hist = [0usize; 256];
    for prog in programs {
        for &b in prog {
            hist[b as usize] += 1;
        }
    }
    hist
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hoe_random_data_near_one() {
        // Pseudorandom data should be near-incompressible.
        use rand::Rng;
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(12345);
        let data: Vec<u8> = (0..8192).map(|_| rng.r#gen()).collect();
        let hoe = high_order_entropy(&data);
        assert!(hoe > 0.9, "HOE of random data should be near 1.0, got {hoe}");
    }

    #[test]
    fn test_hoe_repeated_data_low() {
        // All-same-byte data should be highly compressible.
        let data = vec![42u8; 8192];
        let hoe = high_order_entropy(&data);
        assert!(hoe < 0.1, "HOE of repeated data should be well below 1.0, got {hoe}");
    }

    #[test]
    fn test_hoe_empty() {
        assert_eq!(high_order_entropy(&[]), 0.0);
    }

    #[test]
    fn test_unique_program_count_all_different() {
        let programs: Vec<Vec<u8>> = (0..10u8).map(|i| vec![i; 4]).collect();
        assert_eq!(unique_program_count(&programs), 10);
    }

    #[test]
    fn test_unique_program_count_all_same() {
        let programs: Vec<Vec<u8>> = (0..100).map(|_| vec![42u8; 4]).collect();
        assert_eq!(unique_program_count(&programs), 1);
    }

    #[test]
    fn test_unique_program_count_empty() {
        let programs: Vec<Vec<u8>> = vec![];
        assert_eq!(unique_program_count(&programs), 0);
    }

    #[test]
    fn test_zero_byte_count() {
        let programs = vec![vec![0u8, 1, 0, 2], vec![0, 0, 0, 3]];
        assert_eq!(zero_byte_count(&programs), 5);
    }

    #[test]
    fn test_zero_byte_count_none() {
        let programs = vec![vec![1u8, 2, 3], vec![4, 5, 6]];
        assert_eq!(zero_byte_count(&programs), 0);
    }

    #[test]
    fn test_byte_frequency_histogram() {
        let programs = vec![vec![0u8, 0, 1, 255], vec![0, 1, 1, 2]];
        let hist = byte_frequency_histogram(&programs);
        assert_eq!(hist[0], 3);
        assert_eq!(hist[1], 3);
        assert_eq!(hist[2], 1);
        assert_eq!(hist[255], 1);
        // All others should be 0
        assert_eq!(hist[3], 0);
        assert_eq!(hist[128], 0);
    }

    #[test]
    fn test_byte_frequency_histogram_empty() {
        let programs: Vec<Vec<u8>> = vec![];
        let hist = byte_frequency_histogram(&programs);
        assert_eq!(hist.iter().sum::<usize>(), 0);
    }
}
