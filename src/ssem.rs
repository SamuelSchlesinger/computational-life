use crate::substrate::Substrate;

/// The Manchester Baby (SSEM) substrate — the world's first stored-program computer (1948).
///
/// The Small-Scale Experimental Machine, built at the University of Manchester by
/// Frederic Williams, Tom Kilburn, and Geoff Tootill, ran its first program on
/// 21 June 1948. It had 32 words of 32-bit storage implemented with Williams tubes
/// (CRT-based memory), a single accumulator register, and seven instructions.
///
/// This adaptation maps the SSEM onto a byte tape. The tape is treated as an
/// array of 32-bit little-endian words (4 bytes per word). The number of words
/// is `tape.len() / 4`; any trailing bytes that do not complete a word are unused.
///
/// State:
/// - `pc`: program counter (Control Instruction), starts at 0
/// - `acc`: accumulator (i32, wrapping arithmetic), starts at 0
///
/// Instruction encoding (per 4-byte word):
/// - Byte 0, bits [2:0]: opcode (0-7)
/// - Byte 1: operand address (modulo number of words)
/// - Bytes 2-3: unused for instruction decode (but part of the 32-bit word for data)
///
/// Opcodes:
/// - 0 (JMP):  CI = Store[S]
/// - 1 (JRP):  CI += Store[S]
/// - 2 (LDN):  A = -Store[S]
/// - 3 (STO):  Store[S] = A
/// - 4 (SUB):  A -= Store[S]
/// - 5 (CMP):  if A < 0, skip next instruction
/// - 6 (STOP): halt
/// - 7 (NOP):  no operation
pub struct Ssem;

// Opcodes (3 bits from low bits of first byte of each word)
const JMP: u8 = 0;
const JRP: u8 = 1;
const LDN: u8 = 2;
const STO: u8 = 3;
const SUB: u8 = 4;
const CMP: u8 = 5;
const STOP: u8 = 6;
const NOP: u8 = 7;

/// Read a 32-bit little-endian word from the tape at the given word address.
/// All byte indices wrap modulo tape length.
fn read_word(tape: &[u8], word_addr: usize, num_words: usize) -> i32 {
    let addr = word_addr % num_words;
    let base = addr * 4;
    let b0 = tape[base] as u32;
    let b1 = tape[base + 1] as u32;
    let b2 = tape[base + 2] as u32;
    let b3 = tape[base + 3] as u32;
    (b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)) as i32
}

/// Write a 32-bit little-endian word to the tape at the given word address.
fn write_word(tape: &mut [u8], word_addr: usize, num_words: usize, value: i32) {
    let addr = word_addr % num_words;
    let base = addr * 4;
    let v = value as u32;
    tape[base] = v as u8;
    tape[base + 1] = (v >> 8) as u8;
    tape[base + 2] = (v >> 16) as u8;
    tape[base + 3] = (v >> 24) as u8;
}

impl Substrate for Ssem {
    fn execute(tape: &mut [u8], step_limit: usize) -> usize {
        let num_words = tape.len() / 4;
        if num_words == 0 {
            return 0;
        }

        let mut pc: usize = 0;
        let mut acc: i32 = 0;
        let mut steps: usize = 0;

        while steps < step_limit {
            steps += 1;

            let base = (pc % num_words) * 4;
            let opcode = tape[base] & 0x07;
            let operand = tape[base + 1] as usize % num_words;

            match opcode {
                JMP => {
                    // CI = Store[S] — set PC to value at address S
                    let target = read_word(tape, operand, num_words);
                    pc = (target as usize) % num_words;
                }
                JRP => {
                    // CI += Store[S] — relative jump
                    let offset = read_word(tape, operand, num_words);
                    pc = (pc as i32).wrapping_add(offset) as usize % num_words;
                }
                LDN => {
                    // A = -Store[S] — load negated
                    let val = read_word(tape, operand, num_words);
                    acc = val.wrapping_neg();
                    pc = (pc + 1) % num_words;
                }
                STO => {
                    // Store[S] = A — store accumulator
                    write_word(tape, operand, num_words, acc);
                    pc = (pc + 1) % num_words;
                }
                SUB => {
                    // A -= Store[S] — subtract
                    let val = read_word(tape, operand, num_words);
                    acc = acc.wrapping_sub(val);
                    pc = (pc + 1) % num_words;
                }
                CMP => {
                    // if A < 0 then skip next instruction
                    if acc < 0 {
                        pc = (pc + 2) % num_words;
                    } else {
                        pc = (pc + 1) % num_words;
                    }
                }
                STOP => {
                    // Halt execution
                    break;
                }
                NOP | _ => {
                    // NOP — advance PC
                    pc = (pc + 1) % num_words;
                }
            }
        }

        steps
    }

    fn is_instruction(byte: u8) -> bool {
        // A byte is a meaningful instruction if its low 3 bits encode opcode 0-6
        (byte & 0x07) <= STOP
    }

    fn disassemble(tape: &[u8]) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        let num_words = tape.len() / 4;

        for word_idx in 0..num_words {
            let base = word_idx * 4;
            let b0 = tape[base];
            let b1 = tape[base + 1];
            let b2 = tape[base + 2];
            let b3 = tape[base + 3];
            let opcode = b0 & 0x07;
            let operand = b1 as usize % num_words.max(1);

            let mnemonic = match opcode {
                JMP => format!("JMP [{}]", operand),
                JRP => format!("JRP [{}]", operand),
                LDN => format!("LDN [{}]", operand),
                STO => format!("STO [{}]", operand),
                SUB => format!("SUB [{}]", operand),
                CMP => "CMP".to_string(),
                STOP => "STOP".to_string(),
                _ => "NOP".to_string(),
            };
            let _ = writeln!(
                out,
                "{word_idx:04X}: {b0:02X} {b1:02X} {b2:02X} {b3:02X}  {mnemonic}"
            );
        }

        // Trailing bytes that don't form a complete word
        let trailing_start = num_words * 4;
        for (i, &byte) in tape.iter().enumerate().skip(trailing_start) {
            let _ = writeln!(out, "{i:04X}: {byte:02X}        (trailing)");
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a tape of the given size, copying program bytes to the front.
    fn make_tape(program: &[u8], size: usize) -> Vec<u8> {
        let mut tape = vec![0u8; size];
        for (i, &b) in program.iter().enumerate() {
            if i < size {
                tape[i] = b;
            }
        }
        tape
    }

    /// Build a 4-byte word from opcode and operand address.
    fn word(opcode: u8, operand: u8) -> [u8; 4] {
        [opcode & 0x07, operand, 0, 0]
    }

    /// Build a 4-byte data word from an i32 value (little-endian).
    fn data(value: i32) -> [u8; 4] {
        let v = value as u32;
        [v as u8, (v >> 8) as u8, (v >> 16) as u8, (v >> 24) as u8]
    }

    /// Build a tape from a list of 4-byte words.
    fn tape_from_words(words: &[[u8; 4]]) -> Vec<u8> {
        words.iter().flat_map(|w| w.iter().copied()).collect()
    }

    // --- Individual instruction tests ---

    #[test]
    fn test_stop() {
        // Word 0: STOP
        let mut tape = tape_from_words(&[word(STOP, 0), data(0)]);
        let steps = Ssem::execute(&mut tape, 8192);
        assert_eq!(steps, 1);
    }

    #[test]
    fn test_ldn() {
        // Word 0: LDN [2]   — acc = -Store[2]
        // Word 1: STOP
        // Word 2: data(42)  — Store[2] = 42
        // After LDN: acc = -42
        let mut tape = tape_from_words(&[word(LDN, 2), word(STOP, 0), data(42)]);
        let steps = Ssem::execute(&mut tape, 8192);
        assert_eq!(steps, 2);
        // Verify accumulator value indirectly by storing it
    }

    #[test]
    fn test_ldn_and_sto() {
        // Word 0: LDN [3]   — acc = -Store[3] = -100
        // Word 1: STO [4]   — Store[4] = acc = -100
        // Word 2: STOP
        // Word 3: data(100)
        // Word 4: data(0)   — will be overwritten
        let mut tape = tape_from_words(&[
            word(LDN, 3),
            word(STO, 4),
            word(STOP, 0),
            data(100),
            data(0),
        ]);
        Ssem::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 4, 5), -100);
    }

    #[test]
    fn test_sub() {
        // Word 0: LDN [4]   — acc = -Store[4] = -10
        // Word 1: SUB [5]   — acc = -10 - 3 = -13
        // Word 2: STO [6]   — Store[6] = acc = -13
        // Word 3: STOP
        // Word 4: data(10)
        // Word 5: data(3)
        // Word 6: data(0)   — will be overwritten
        let mut tape = tape_from_words(&[
            word(LDN, 4),
            word(SUB, 5),
            word(STO, 6),
            word(STOP, 0),
            data(10),
            data(3),
            data(0),
        ]);
        Ssem::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 6, 7), -13);
    }

    #[test]
    fn test_jmp() {
        // Word 0: JMP [3]   — pc = Store[3] = 2
        // Word 1: STO [4]   — should be skipped
        // Word 2: STOP
        // Word 3: data(2)   — jump target value
        // Word 4: data(0)   — should remain 0 if STO is skipped
        let mut tape = tape_from_words(&[
            word(JMP, 3),
            word(STO, 4),
            word(STOP, 0),
            data(2),
            data(0),
        ]);
        Ssem::execute(&mut tape, 8192);
        // Word 4 should still be 0 because STO at word 1 was skipped
        assert_eq!(read_word(&tape, 4, 5), 0);
    }

    #[test]
    fn test_jrp() {
        // Word 0: LDN [4]   — acc = -42
        // Word 1: JRP [4]   — pc += Store[4] = pc + 2 = 1 + 2 = 3
        // Word 2: STO [5]   — should be skipped
        // Word 3: STOP
        // Word 4: data(2)   — offset value
        // Word 5: data(0)   — should remain 0 if STO at word 2 is skipped
        let mut tape = tape_from_words(&[
            word(LDN, 4),
            word(JRP, 4),
            word(STO, 5),
            word(STOP, 0),
            data(2),
            data(0),
        ]);
        Ssem::execute(&mut tape, 8192);
        // STO at word 2 was skipped, so word 5 remains 0
        assert_eq!(read_word(&tape, 5, 6), 0);
    }

    #[test]
    fn test_cmp_negative() {
        // Word 0: LDN [4]   — acc = -10 (negated 10), acc is negative
        // Word 1: CMP       — acc < 0, so skip word 2
        // Word 2: STO [5]   — should be skipped
        // Word 3: STOP
        // Word 4: data(10)
        // Word 5: data(0)   — should remain 0
        let mut tape = tape_from_words(&[
            word(LDN, 4),
            word(CMP, 0),
            word(STO, 5),
            word(STOP, 0),
            data(10),
            data(0),
        ]);
        Ssem::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 5, 6), 0); // STO was skipped
    }

    #[test]
    fn test_cmp_non_negative() {
        // Word 0: LDN [4]   — acc = 0 (negated 0), acc is zero (not negative)
        // Word 1: CMP       — acc >= 0, do NOT skip
        // Word 2: STO [5]   — should execute, storing acc (0) to word 5
        // Word 3: STOP
        // Word 4: data(0)
        // Word 5: data(99)  — should be overwritten with 0
        let mut tape = tape_from_words(&[
            word(LDN, 4),
            word(CMP, 0),
            word(STO, 5),
            word(STOP, 0),
            data(0),
            data(99),
        ]);
        Ssem::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 5, 6), 0); // STO executed, wrote acc=0
    }

    #[test]
    fn test_nop() {
        // Word 0: NOP (opcode 7)
        // Word 1: STOP
        let mut tape = tape_from_words(&[word(NOP, 0), word(STOP, 0)]);
        let steps = Ssem::execute(&mut tape, 8192);
        assert_eq!(steps, 2); // NOP + STOP
    }

    // --- Edge case tests ---

    #[test]
    fn test_empty_tape() {
        let mut tape: Vec<u8> = vec![];
        let steps = Ssem::execute(&mut tape, 8192);
        assert_eq!(steps, 0);
    }

    #[test]
    fn test_tape_shorter_than_4_bytes() {
        let mut tape = make_tape(&[0, 0, 0], 3);
        let steps = Ssem::execute(&mut tape, 8192);
        assert_eq!(steps, 0);
    }

    #[test]
    fn test_step_limit() {
        // Word 0: JMP [1]   — pc = Store[1] = 0. Infinite loop.
        // Word 1: data(0)
        let mut tape = make_tape(&[JMP, 1, 0, 0, 0, 0, 0, 0], 8);
        let steps = Ssem::execute(&mut tape, 100);
        assert_eq!(steps, 100);
    }

    #[test]
    fn test_wrapping_arithmetic() {
        // LDN on i32::MIN should wrap: -i32::MIN overflows to i32::MIN in wrapping
        // Word 0: LDN [2]   — acc = -(i32::MIN) = i32::MIN (wrapping)
        // Word 1: STO [3]
        // Word 2: data(i32::MIN)
        // Word 3: data(0)
        // Word 4: STOP
        let mut tape = tape_from_words(&[
            word(LDN, 2),
            word(STO, 3),
            data(i32::MIN),
            data(0),
            word(STOP, 0),
        ]);
        Ssem::execute(&mut tape, 8192);
        // wrapping_neg of i32::MIN is i32::MIN
        assert_eq!(read_word(&tape, 3, 5), i32::MIN);
    }

    #[test]
    fn test_address_wraps_modulo_num_words() {
        // 4 words total. Operand = 200, which wraps to 200 % 4 = 0.
        // Word 0: LDN with operand 200 (wraps to 0)
        // Word 1: STO [2]
        // Word 2: data(0)
        // Word 3: STOP
        let mut tape = tape_from_words(&[
            [LDN, 200, 0, 0],  // operand 200 % 4 = 0
            word(STO, 2),
            data(0),
            word(STOP, 0),
        ]);
        // LDN [0] reads word 0, negates it. Word 0 contains [LDN, 200, 0, 0].
        // As i32 LE: LDN=2, 200, 0, 0 => 0x0000C802 = 51202. Negated = -51202.
        Ssem::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 2, 4), -51202);
    }

    #[test]
    fn test_is_instruction() {
        // Opcodes 0-6 are meaningful instructions
        for opcode in 0..=6u8 {
            assert!(Ssem::is_instruction(opcode), "opcode {} should be instruction", opcode);
        }
        // Opcode 7 (NOP) is not meaningful
        assert!(!Ssem::is_instruction(7));
        // Higher bits don't matter: 0b11111_000 = 0xF8 has low 3 bits = 0 (JMP)
        assert!(Ssem::is_instruction(0xF8));
        // 0b11111_111 = 0xFF has low 3 bits = 7 (NOP)
        assert!(!Ssem::is_instruction(0xFF));
    }

    #[test]
    fn test_disassemble() {
        let tape = tape_from_words(&[
            word(JMP, 5),
            word(JRP, 3),
            word(LDN, 7),
            word(STO, 2),
            word(SUB, 1),
            word(CMP, 0),
            word(STOP, 0),
            word(NOP, 0),
        ]);
        let dis = Ssem::disassemble(&tape);
        assert!(dis.contains("JMP"));
        assert!(dis.contains("JRP"));
        assert!(dis.contains("LDN"));
        assert!(dis.contains("STO"));
        assert!(dis.contains("SUB"));
        assert!(dis.contains("CMP"));
        assert!(dis.contains("STOP"));
        assert!(dis.contains("NOP"));
    }

    #[test]
    fn test_disassemble_trailing_bytes() {
        // 2 words + 2 trailing bytes
        let mut tape = tape_from_words(&[word(STOP, 0), word(NOP, 0)]);
        tape.push(0xAB);
        tape.push(0xCD);
        let dis = Ssem::disassemble(&tape);
        assert!(dis.contains("trailing"));
    }

    #[test]
    fn test_sub_wrapping() {
        // acc = 0, SUB 10 => acc = 0 - 10 = -10
        // Then SUB i32::MAX => acc = -10 - i32::MAX = wrapping
        let mut tape = tape_from_words(&[
            word(SUB, 4),
            word(SUB, 5),
            word(STO, 6),
            word(STOP, 0),
            data(10),
            data(i32::MAX),
            data(0),
        ]);
        Ssem::execute(&mut tape, 8192);
        let expected = (0i32).wrapping_sub(10).wrapping_sub(i32::MAX);
        assert_eq!(read_word(&tape, 6, 7), expected);
    }

    #[test]
    fn test_pc_wraps_around() {
        // 2 words. Word 0: NOP, Word 1: STOP.
        // PC starts at 0, NOP advances to 1, STOP halts. Normal.
        // Now test wrap: 2 words, word 0: NOP, word 1: NOP.
        // After word 1, pc = (1+1) % 2 = 0. Loops.
        let mut tape = tape_from_words(&[word(NOP, 0), word(NOP, 0)]);
        let steps = Ssem::execute(&mut tape, 100);
        assert_eq!(steps, 100);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn random_programs_never_panic(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let mut tape = tape_data;
            let steps = Ssem::execute(&mut tape, 8192);
            prop_assert!(steps <= 8192);
        }

        #[test]
        fn random_programs_respect_step_limit(
            tape_data in prop::collection::vec(any::<u8>(), 1..256),
            limit in 1usize..1000
        ) {
            let mut tape = tape_data;
            let steps = Ssem::execute(&mut tape, limit);
            prop_assert!(steps <= limit);
        }

        #[test]
        fn output_tape_same_length(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let original_len = tape_data.len();
            let mut tape = tape_data;
            Ssem::execute(&mut tape, 8192);
            prop_assert_eq!(tape.len(), original_len);
        }
    }
}
