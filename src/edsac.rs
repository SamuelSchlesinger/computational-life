use crate::substrate::Substrate;

/// The EDSAC (Electronic Delay Storage Automatic Calculator) substrate.
///
/// EDSAC, completed in 1949 at the University of Cambridge, was the first
/// practical stored-program computer — the first machine to run a program
/// loaded entirely from its own memory rather than from plugboards or paper
/// tape. Its mercury delay-line memory stored both instructions and data in
/// the same address space, establishing the von Neumann architecture that
/// underpins nearly every computer built since.
///
/// This substrate maps EDSAC onto a byte tape using 2-byte words:
/// - First byte of each instruction word: opcode (low 5 bits)
/// - Second byte: operand address (word index, modulo num_words)
/// - Data words are stored as little-endian i16 with wrapping arithmetic
///
/// State:
/// - `pc`: program counter (word index), starts at 0
/// - `acc`: accumulator (i16, wrapping), starts at 0
/// - Number of words = tape.len() / 2
pub struct Edsac;

// Opcodes (low 5 bits of the first byte of each 2-byte instruction word)
const HALT: u8 = 0x00;
const ADD: u8 = 0x01;
const SUB: u8 = 0x02;
const LOAD: u8 = 0x03;
const STORE: u8 = 0x04;
const LOAD_NEG: u8 = 0x05;
const AND: u8 = 0x06;
const SHIFT_L: u8 = 0x07;
const SHIFT_R: u8 = 0x08;
const JMP: u8 = 0x09;
const JN: u8 = 0x0A;
const NOP: u8 = 0x0B;
const STORE_CLR: u8 = 0x0C;
const MULT_ADD: u8 = 0x0D;

/// Read a little-endian i16 word from the tape at the given word address.
fn read_word(tape: &[u8], word_addr: usize, num_words: usize) -> i16 {
    let offset = (word_addr % num_words) * 2;
    i16::from_le_bytes([tape[offset], tape[offset + 1]])
}

/// Write a little-endian i16 word to the tape at the given word address.
fn write_word(tape: &mut [u8], word_addr: usize, num_words: usize, val: i16) {
    let offset = (word_addr % num_words) * 2;
    let bytes = val.to_le_bytes();
    tape[offset] = bytes[0];
    tape[offset + 1] = bytes[1];
}

impl Substrate for Edsac {
    fn execute(tape: &mut [u8], step_limit: usize) -> usize {
        let num_words = tape.len() / 2;
        if num_words == 0 {
            return 0;
        }

        let mut pc: usize = 0;
        let mut acc: i16 = 0;
        let mut steps: usize = 0;

        while steps < step_limit {
            steps += 1;

            let byte_offset = (pc % num_words) * 2;
            let opcode = tape[byte_offset] & 0x1F;
            let operand = tape[byte_offset + 1] as usize;

            match opcode {
                HALT => break,
                ADD => {
                    let val = read_word(tape, operand, num_words);
                    acc = acc.wrapping_add(val);
                }
                SUB => {
                    let val = read_word(tape, operand, num_words);
                    acc = acc.wrapping_sub(val);
                }
                LOAD => {
                    acc = read_word(tape, operand, num_words);
                }
                STORE => {
                    write_word(tape, operand, num_words, acc);
                }
                LOAD_NEG => {
                    let val = read_word(tape, operand, num_words);
                    acc = val.wrapping_neg();
                }
                AND => {
                    let val = read_word(tape, operand, num_words);
                    acc &= val;
                }
                SHIFT_L => {
                    let shift = (operand as u32) & 0x0F;
                    acc = acc.wrapping_shl(shift);
                }
                SHIFT_R => {
                    let shift = (operand as u32) & 0x0F;
                    acc = acc.wrapping_shr(shift);
                }
                JMP => {
                    pc = operand % num_words;
                    continue;
                }
                JN => {
                    if acc < 0 {
                        pc = operand % num_words;
                        continue;
                    }
                }
                NOP => {}
                STORE_CLR => {
                    write_word(tape, operand, num_words, acc);
                    acc = 0;
                }
                MULT_ADD => {
                    let val = read_word(tape, operand, num_words);
                    acc = acc.wrapping_add(acc.wrapping_mul(val));
                }
                _ => {} // 0x0E-0x1F: NOP
            }

            pc += 1;
        }

        steps
    }

    fn is_instruction(byte: u8) -> bool {
        (byte & 0x1F) <= MULT_ADD
    }

    fn disassemble(tape: &[u8]) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        let num_words = tape.len() / 2;
        for w in 0..num_words {
            let offset = w * 2;
            let b0 = tape[offset];
            let b1 = tape[offset + 1];
            let opcode = b0 & 0x1F;
            let mnemonic = match opcode {
                HALT => "HALT".to_string(),
                ADD => format!("ADD {b1}"),
                SUB => format!("SUB {b1}"),
                LOAD => format!("LOAD {b1}"),
                STORE => format!("STORE {b1}"),
                LOAD_NEG => format!("LOAD_NEG {b1}"),
                AND => format!("AND {b1}"),
                SHIFT_L => format!("SHIFT_L {}", b1 & 0x0F),
                SHIFT_R => format!("SHIFT_R {}", b1 & 0x0F),
                JMP => format!("JMP {b1}"),
                JN => format!("JN {b1}"),
                NOP => "NOP".to_string(),
                STORE_CLR => format!("STORE_CLR {b1}"),
                MULT_ADD => format!("MULT_ADD {b1}"),
                _ => "NOP".to_string(),
            };
            let _ = writeln!(out, "{offset:04X}: {b0:02X} {b1:02X}  {mnemonic}");
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a tape of the given size, placing `program` bytes at the start.
    fn make_tape(program: &[u8], size: usize) -> Vec<u8> {
        let mut tape = vec![0u8; size];
        for (i, &b) in program.iter().enumerate() {
            if i < size {
                tape[i] = b;
            }
        }
        tape
    }

    // Helper: encode a 2-byte instruction word.
    fn instr(opcode: u8, operand: u8) -> [u8; 2] {
        [opcode, operand]
    }

    // Helper: encode an i16 value as two little-endian bytes.
    fn word(val: i16) -> [u8; 2] {
        val.to_le_bytes()
    }

    #[test]
    fn test_empty_tape() {
        let mut tape: Vec<u8> = vec![];
        let steps = Edsac::execute(&mut tape, 8192);
        assert_eq!(steps, 0);
    }

    #[test]
    fn test_single_byte_tape() {
        let mut tape: Vec<u8> = vec![0x42];
        let steps = Edsac::execute(&mut tape, 8192);
        assert_eq!(steps, 0);
    }

    #[test]
    fn test_halt() {
        // Word 0: HALT
        let mut tape = make_tape(&instr(HALT, 0), 128);
        let steps = Edsac::execute(&mut tape, 8192);
        assert_eq!(steps, 1);
    }

    #[test]
    fn test_add() {
        // Word 0: LOAD 2    — acc = mem[2]
        // Word 1: ADD 3     — acc += mem[3]
        // Word 2: STORE 4   — mem[4] = acc
        // Word 3: HALT
        // Word 4 (addr 2): data = 10
        // Word 5 (addr 3): data = 25
        // Word 6 (addr 4): result destination
        let mut tape = vec![0u8; 14]; // 7 words
        let i0 = instr(LOAD, 4);
        let i1 = instr(ADD, 5);
        let i2 = instr(STORE, 6);
        let i3 = instr(HALT, 0);
        let d0 = word(10);
        let d1 = word(25);
        tape[0..2].copy_from_slice(&i0);
        tape[2..4].copy_from_slice(&i1);
        tape[4..6].copy_from_slice(&i2);
        tape[6..8].copy_from_slice(&i3);
        tape[8..10].copy_from_slice(&d0);
        tape[10..12].copy_from_slice(&d1);

        Edsac::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 6, 7), 35);
    }

    #[test]
    fn test_sub() {
        // Load 100, subtract 30, store result.
        let mut tape = vec![0u8; 14]; // 7 words
        tape[0..2].copy_from_slice(&instr(LOAD, 4));
        tape[2..4].copy_from_slice(&instr(SUB, 5));
        tape[4..6].copy_from_slice(&instr(STORE, 6));
        tape[6..8].copy_from_slice(&instr(HALT, 0));
        tape[8..10].copy_from_slice(&word(100));
        tape[10..12].copy_from_slice(&word(30));

        Edsac::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 6, 7), 70);
    }

    #[test]
    fn test_load() {
        // LOAD 2, STORE 3, HALT. Word 2 = 42.
        let mut tape = vec![0u8; 8]; // 4 words
        tape[0..2].copy_from_slice(&instr(LOAD, 2));
        tape[2..4].copy_from_slice(&instr(STORE, 3));
        tape[4..6].copy_from_slice(&instr(HALT, 0));
        // Word 2 is at byte offset 4 — but that's our HALT instruction.
        // Use a bigger tape:
        let mut tape = vec![0u8; 10]; // 5 words
        tape[0..2].copy_from_slice(&instr(LOAD, 3));
        tape[2..4].copy_from_slice(&instr(STORE, 4));
        tape[4..6].copy_from_slice(&instr(HALT, 0));
        tape[6..8].copy_from_slice(&word(42));

        Edsac::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 4, 5), 42);
    }

    #[test]
    fn test_store() {
        // LOAD 3, STORE 4, HALT. Word 3 = -7.
        let mut tape = vec![0u8; 10]; // 5 words
        tape[0..2].copy_from_slice(&instr(LOAD, 3));
        tape[2..4].copy_from_slice(&instr(STORE, 4));
        tape[4..6].copy_from_slice(&instr(HALT, 0));
        tape[6..8].copy_from_slice(&word(-7));

        Edsac::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 4, 5), -7);
    }

    #[test]
    fn test_load_neg() {
        // LOAD_NEG 3, STORE 4, HALT. Word 3 = 17.
        let mut tape = vec![0u8; 10]; // 5 words
        tape[0..2].copy_from_slice(&instr(LOAD_NEG, 3));
        tape[2..4].copy_from_slice(&instr(STORE, 4));
        tape[4..6].copy_from_slice(&instr(HALT, 0));
        tape[6..8].copy_from_slice(&word(17));

        Edsac::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 4, 5), -17);
    }

    #[test]
    fn test_and() {
        // LOAD 3 (0xFF00), AND 4 (0x0F0F), STORE 5, HALT.
        let mut tape = vec![0u8; 12]; // 6 words
        tape[0..2].copy_from_slice(&instr(LOAD, 3));
        tape[2..4].copy_from_slice(&instr(AND, 4));
        tape[4..6].copy_from_slice(&instr(STORE, 5));
        // HALT is needed — put it in the program flow but we have 6 words starting at 0.
        // Actually words 0,1,2 are instructions. We need HALT at word index 3? No, 3 is data.
        // Rearrange: 4 instructions + 3 data = 7 words.
        let mut tape = vec![0u8; 14]; // 7 words
        tape[0..2].copy_from_slice(&instr(LOAD, 4));
        tape[2..4].copy_from_slice(&instr(AND, 5));
        tape[4..6].copy_from_slice(&instr(STORE, 6));
        tape[6..8].copy_from_slice(&instr(HALT, 0));
        write_word(&mut tape, 4, 7, 0x7F00_u16 as i16); // -256 in i16 would be 0xFF00, let's use explicit
        write_word(&mut tape, 5, 7, 0x0F0F);

        Edsac::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 6, 7), 0x0F00);
    }

    #[test]
    fn test_shift_l() {
        // LOAD 2, SHIFT_L 3, STORE 3, HALT. Word 2 = 1. Shift left by 3.
        let mut tape = vec![0u8; 10]; // 5 words
        tape[0..2].copy_from_slice(&instr(LOAD, 3));
        tape[2..4].copy_from_slice(&instr(SHIFT_L, 3)); // shift by 3
        tape[4..6].copy_from_slice(&instr(STORE, 4));
        tape[6..8].copy_from_slice(&word(1));
        // Need HALT — extend to 6 words.
        let mut tape = vec![0u8; 12]; // 6 words
        tape[0..2].copy_from_slice(&instr(LOAD, 4));
        tape[2..4].copy_from_slice(&instr(SHIFT_L, 3));
        tape[4..6].copy_from_slice(&instr(STORE, 5));
        tape[6..8].copy_from_slice(&instr(HALT, 0));
        tape[8..10].copy_from_slice(&word(1));

        Edsac::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 5, 6), 8); // 1 << 3 = 8
    }

    #[test]
    fn test_shift_r() {
        // LOAD word(-16), SHIFT_R 2, STORE, HALT.
        // -16 in i16 is 0xFFF0. Arithmetic right shift by 2 = 0xFFFC = -4.
        let mut tape = vec![0u8; 12]; // 6 words
        tape[0..2].copy_from_slice(&instr(LOAD, 4));
        tape[2..4].copy_from_slice(&instr(SHIFT_R, 2));
        tape[4..6].copy_from_slice(&instr(STORE, 5));
        tape[6..8].copy_from_slice(&instr(HALT, 0));
        tape[8..10].copy_from_slice(&word(-16));

        Edsac::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 5, 6), -4); // -16 >> 2 = -4 (arithmetic)
    }

    #[test]
    fn test_jmp() {
        // Word 0: JMP 2 — jump to word 2
        // Word 1: HALT   — should be skipped
        // Word 2: LOAD 4 — acc = 99
        // Word 3: STORE 5
        // Word 4: HALT
        // Word 5 (addr 4): 99
        let mut tape = vec![0u8; 12]; // 6 words
        tape[0..2].copy_from_slice(&instr(JMP, 2));
        tape[2..4].copy_from_slice(&instr(HALT, 0)); // skipped
        tape[4..6].copy_from_slice(&instr(LOAD, 5));
        tape[6..8].copy_from_slice(&instr(STORE, 4)); // store acc to word 4
        tape[8..10].copy_from_slice(&instr(HALT, 0));
        tape[10..12].copy_from_slice(&word(99));

        Edsac::execute(&mut tape, 8192);
        // acc loaded 99 from word 5, stored to word 4 (byte offset 8)
        assert_eq!(read_word(&tape, 4, 6), 99);
    }

    #[test]
    fn test_jn_taken() {
        // acc = -1 (negative). JN should jump.
        let mut tape = vec![0u8; 12]; // 6 words
        tape[0..2].copy_from_slice(&instr(LOAD, 4));  // acc = -1
        tape[2..4].copy_from_slice(&instr(JN, 5));    // acc < 0, jump to word 5
        tape[4..6].copy_from_slice(&instr(STORE, 3)); // skipped
        tape[6..8].copy_from_slice(&instr(HALT, 0));
        tape[8..10].copy_from_slice(&word(-1));
        tape[10..12].copy_from_slice(&instr(HALT, 0));

        let steps = Edsac::execute(&mut tape, 8192);
        // LOAD, JN (taken -> word 5), HALT = 3 steps
        assert_eq!(steps, 3);
        // Word 3 should NOT have been written (STORE was skipped)
        assert_eq!(read_word(&tape, 3, 6), 0);
    }

    #[test]
    fn test_jn_not_taken() {
        // acc = 5 (positive). JN should not jump.
        let mut tape = vec![0u8; 12]; // 6 words
        tape[0..2].copy_from_slice(&instr(LOAD, 4));  // acc = 5
        tape[2..4].copy_from_slice(&instr(JN, 5));    // acc >= 0, fall through
        tape[4..6].copy_from_slice(&instr(STORE, 5)); // should execute
        tape[6..8].copy_from_slice(&instr(HALT, 0));
        tape[8..10].copy_from_slice(&word(5));

        Edsac::execute(&mut tape, 8192);
        // STORE should have written acc=5 to word 5
        assert_eq!(read_word(&tape, 5, 6), 5);
    }

    #[test]
    fn test_jn_zero_not_taken() {
        // acc = 0. JN should NOT jump (0 is not negative).
        let mut tape = vec![0u8; 10]; // 5 words
        tape[0..2].copy_from_slice(&instr(LOAD, 3));  // acc = 0
        tape[2..4].copy_from_slice(&instr(JN, 0));    // acc >= 0, fall through
        tape[4..6].copy_from_slice(&instr(STORE, 4)); // should execute
        tape[6..8].copy_from_slice(&word(0));

        Edsac::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 4, 5), 0);
    }

    #[test]
    fn test_nop() {
        // NOP, HALT. Should take 2 steps.
        let mut tape = vec![0u8; 4]; // 2 words
        tape[0..2].copy_from_slice(&instr(NOP, 0));
        tape[2..4].copy_from_slice(&instr(HALT, 0));

        let steps = Edsac::execute(&mut tape, 8192);
        assert_eq!(steps, 2);
    }

    #[test]
    fn test_store_clr() {
        // LOAD 3, STORE_CLR 4, STORE 3, HALT.
        // After STORE_CLR: mem[4] = acc, acc = 0.
        // Then STORE writes acc (now 0) to word 3.
        let mut tape = vec![0u8; 10]; // 5 words
        tape[0..2].copy_from_slice(&instr(LOAD, 4));
        tape[2..4].copy_from_slice(&instr(STORE_CLR, 3));
        tape[4..6].copy_from_slice(&instr(STORE, 4)); // store 0 (cleared acc) back to word 4
        tape[6..8].copy_from_slice(&instr(HALT, 0));
        tape[8..10].copy_from_slice(&word(77));

        Edsac::execute(&mut tape, 8192);
        // Word 3 should have the original acc value (77)
        assert_eq!(read_word(&tape, 3, 5), 77);
        // Word 4 should be 0 (acc was cleared, then stored)
        assert_eq!(read_word(&tape, 4, 5), 0);
    }

    #[test]
    fn test_mult_add() {
        // LOAD 3 (acc=5), MULT_ADD 4 (acc += acc * mem[4] = 5 + 5*3 = 20), STORE 3, HALT.
        let mut tape = vec![0u8; 12]; // 6 words
        tape[0..2].copy_from_slice(&instr(LOAD, 4));
        tape[2..4].copy_from_slice(&instr(MULT_ADD, 5));
        tape[4..6].copy_from_slice(&instr(STORE, 3));
        tape[6..8].copy_from_slice(&instr(HALT, 0));
        tape[8..10].copy_from_slice(&word(5));
        tape[10..12].copy_from_slice(&word(3));

        Edsac::execute(&mut tape, 8192);
        // acc = 5 + 5 * 3 = 20
        assert_eq!(read_word(&tape, 3, 6), 20);
    }

    #[test]
    fn test_wrapping_arithmetic() {
        // i16::MAX + 1 should wrap to i16::MIN.
        let mut tape = vec![0u8; 12]; // 6 words
        tape[0..2].copy_from_slice(&instr(LOAD, 4));
        tape[2..4].copy_from_slice(&instr(ADD, 5));
        tape[4..6].copy_from_slice(&instr(STORE, 3));
        tape[6..8].copy_from_slice(&instr(HALT, 0));
        tape[8..10].copy_from_slice(&word(i16::MAX));
        tape[10..12].copy_from_slice(&word(1));

        Edsac::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 3, 6), i16::MIN);
    }

    #[test]
    fn test_modular_word_addressing() {
        // With 4 words (8 bytes), operand address 7 wraps to 7 % 4 = 3.
        let mut tape = vec![0u8; 8]; // 4 words
        tape[0..2].copy_from_slice(&instr(LOAD, 7));  // 7 % 4 = word 3
        tape[2..4].copy_from_slice(&instr(STORE, 6)); // 6 % 4 = word 2
        tape[4..6].copy_from_slice(&instr(HALT, 0));
        tape[6..8].copy_from_slice(&word(123));

        Edsac::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 2, 4), 123);
    }

    #[test]
    fn test_high_bits_ignored_for_opcode() {
        // Opcode byte 0xE3 has low 5 bits = 0x03 = LOAD. High 3 bits are ignored.
        let mut tape = vec![0u8; 8]; // 4 words
        tape[0] = 0xE3; // 0b11100011 — low 5 bits = LOAD
        tape[1] = 3;    // operand: word 3
        tape[2..4].copy_from_slice(&instr(STORE, 2));
        tape[4..6].copy_from_slice(&instr(HALT, 0));
        tape[6..8].copy_from_slice(&word(55));

        Edsac::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 2, 4), 55);
    }

    #[test]
    fn test_unknown_opcodes_are_nop() {
        // Opcodes 0x0E-0x1F should behave as NOP.
        let mut tape = vec![0u8; 6]; // 3 words
        tape[0..2].copy_from_slice(&instr(0x0E, 0));
        tape[2..4].copy_from_slice(&instr(0x1F, 0));
        tape[4..6].copy_from_slice(&instr(HALT, 0));

        let steps = Edsac::execute(&mut tape, 8192);
        assert_eq!(steps, 3); // two NOPs then HALT
    }

    #[test]
    fn test_step_limit() {
        // Infinite loop: JMP 0
        let mut tape = vec![0u8; 4]; // 2 words
        tape[0..2].copy_from_slice(&instr(JMP, 0));
        tape[2..4].copy_from_slice(&instr(HALT, 0)); // never reached

        let steps = Edsac::execute(&mut tape, 100);
        assert_eq!(steps, 100);
    }

    #[test]
    fn test_pc_wraps_at_end() {
        // PC should wrap around when it walks off the end of memory.
        // 2 words: NOP at 0, NOP at 1. PC advances to 2, wraps to 0. Loop.
        let mut tape = vec![0u8; 4]; // 2 words
        tape[0..2].copy_from_slice(&instr(NOP, 0));
        tape[2..4].copy_from_slice(&instr(NOP, 0));

        let steps = Edsac::execute(&mut tape, 50);
        assert_eq!(steps, 50);
    }

    #[test]
    fn test_is_instruction() {
        // Opcodes 0x00-0x0D should be instructions.
        for op in 0x00..=0x0D {
            assert!(Edsac::is_instruction(op), "opcode {op:#04X} should be an instruction");
        }
        // 0x0E and above (low 5 bits) are not meaningful opcodes.
        for op in 0x0E..=0x1F {
            assert!(!Edsac::is_instruction(op), "opcode {op:#04X} should not be an instruction");
        }
        // High bits should be considered: 0x23 has low 5 bits = 0x03 (LOAD).
        assert!(Edsac::is_instruction(0x23));
        // 0x3F has low 5 bits = 0x1F — not an instruction.
        assert!(!Edsac::is_instruction(0x3F));
    }

    #[test]
    fn test_disassemble_basic() {
        let mut tape = vec![0u8; 8]; // 4 words
        tape[0..2].copy_from_slice(&instr(LOAD, 3));
        tape[2..4].copy_from_slice(&instr(ADD, 2));
        tape[4..6].copy_from_slice(&instr(STORE, 1));
        tape[6..8].copy_from_slice(&instr(HALT, 0));

        let disasm = Edsac::disassemble(&tape);
        assert!(disasm.contains("LOAD 3"));
        assert!(disasm.contains("ADD 2"));
        assert!(disasm.contains("STORE 1"));
        assert!(disasm.contains("HALT"));
    }

    #[test]
    fn test_disassemble_format() {
        // Verify the format: {addr:04X}: {b0:02X} {b1:02X}  {mnemonic}
        let mut tape = vec![0u8; 4]; // 2 words
        tape[0..2].copy_from_slice(&instr(JMP, 0x0A));
        tape[2..4].copy_from_slice(&instr(HALT, 0));

        let disasm = Edsac::disassemble(&tape);
        assert!(disasm.contains("0000: 09 0A  JMP 10"));
        assert!(disasm.contains("0002: 00 00  HALT"));
    }

    #[test]
    fn test_disassemble_empty() {
        let tape: Vec<u8> = vec![];
        let disasm = Edsac::disassemble(&tape);
        assert!(disasm.is_empty());
    }

    #[test]
    fn test_disassemble_single_byte() {
        let tape: Vec<u8> = vec![0x42];
        let disasm = Edsac::disassemble(&tape);
        assert!(disasm.is_empty()); // Not enough bytes for a word
    }

    #[test]
    fn test_counting_program() {
        // A small program that counts: acc starts at 0, adds 1 three times.
        // LOAD 5 (word 5 = 1), ADD 5, ADD 5, STORE 6, HALT
        let mut tape = vec![0u8; 14]; // 7 words
        tape[0..2].copy_from_slice(&instr(LOAD, 5));
        tape[2..4].copy_from_slice(&instr(ADD, 5));
        tape[4..6].copy_from_slice(&instr(ADD, 5));
        tape[6..8].copy_from_slice(&instr(STORE, 6));
        tape[8..10].copy_from_slice(&instr(HALT, 0));
        tape[10..12].copy_from_slice(&word(1));

        Edsac::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 6, 7), 3);
    }

    #[test]
    fn test_conditional_loop() {
        // Subtract 1 each iteration; jump back while negative (which happens
        // when we go past zero via wrapping). Start with acc=3, subtract 1
        // and store each time. After 3 subtractions acc=0, JN not taken, HALT.
        //
        // Word 0: LOAD 6      — acc = counter (starts at 3)
        // Word 1: SUB 7       — acc -= 1
        // Word 2: STORE 6     — save counter
        // Word 3: JN 5        — if acc < 0, jump to HALT (word 5)
        // Word 4: JMP 0       — loop back
        // Word 5: HALT
        // Word 6: data = 3    — counter
        // Word 7: data = 1    — decrement value
        let mut tape = vec![0u8; 16]; // 8 words
        tape[0..2].copy_from_slice(&instr(LOAD, 6));
        tape[2..4].copy_from_slice(&instr(SUB, 7));
        tape[4..6].copy_from_slice(&instr(STORE, 6));
        tape[6..8].copy_from_slice(&instr(JN, 5));
        tape[8..10].copy_from_slice(&instr(JMP, 0));
        tape[10..12].copy_from_slice(&instr(HALT, 0));
        tape[12..14].copy_from_slice(&word(3));
        tape[14..16].copy_from_slice(&word(1));

        let steps = Edsac::execute(&mut tape, 8192);
        // Iterations: 3->2 (no jn), 2->1 (no jn), 1->0 (no jn), 0->-1 (jn taken -> HALT)
        // 4 iterations of (LOAD, SUB, STORE, JN, JMP) = 4*5=20, but last iteration
        // is (LOAD, SUB, STORE, JN->HALT) = 4 + HALT = 5
        // Actually: iteration 1: LOAD,SUB,STORE,JN(not taken),JMP = 5
        //           iteration 2: LOAD,SUB,STORE,JN(not taken),JMP = 5
        //           iteration 3: LOAD,SUB,STORE,JN(not taken),JMP = 5
        //           iteration 4: LOAD,SUB,STORE,JN(taken->HALT) = 4, then HALT = 5
        // Total = 5 + 5 + 5 + 4 + 1 = 20
        // Final counter value should be -1
        assert_eq!(read_word(&tape, 6, 8), -1);
        assert_eq!(steps, 20);
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
            let steps = Edsac::execute(&mut tape, 8192);
            prop_assert!(steps <= 8192);
        }

        #[test]
        fn random_programs_respect_step_limit(
            tape_data in prop::collection::vec(any::<u8>(), 1..256),
            limit in 1usize..1000
        ) {
            let mut tape = tape_data;
            let steps = Edsac::execute(&mut tape, limit);
            prop_assert!(steps <= limit);
        }

        #[test]
        fn output_tape_same_length(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let original_len = tape_data.len();
            let mut tape = tape_data;
            Edsac::execute(&mut tape, 8192);
            prop_assert_eq!(tape.len(), original_len);
        }
    }
}
