use crate::substrate::Substrate;

/// The PDP-1 (1959) substrate — DEC's first minicomputer.
///
/// The PDP-1 ushered in the era of interactive computing and ran Spacewar!,
/// the first video game. It had 18-bit words, a single accumulator, an I/O
/// register, and ~27 instructions.
///
/// For byte-tape practicality, 18-bit words are approximated as 16-bit (i16,
/// little-endian). Instructions are 2 bytes: 1 byte opcode (low 5 bits used)
/// + 1 byte address operand.
///
/// State:
/// - `pc`: program counter (word index, i.e. byte offset / 2), starts at 0
/// - `ac`: accumulator (i16, wrapping arithmetic)
/// - `io`: I/O register (i16)
/// - `overflow`: overflow flag (bool)
/// - Number of words = tape.len() / 2
pub struct Pdp1;

// Opcodes (low 5 bits of the first byte of each 2-byte instruction)
const HALT: u8 = 0x00;
const ADD: u8 = 0x01;
const SUB: u8 = 0x02;
const AND: u8 = 0x03;
const IOR: u8 = 0x04;
const XOR: u8 = 0x05;
const LOAD: u8 = 0x06;
const STORE: u8 = 0x07;
const LOAD_NEG: u8 = 0x08;
const SWAP: u8 = 0x09;
const JMP: u8 = 0x0A;
const JSR: u8 = 0x0B;
const SKP_Z: u8 = 0x0C;
const SKP_POS: u8 = 0x0D;
const SKP_NEG: u8 = 0x0E;
const SKP_OVF: u8 = 0x0F;
const SHIFT_L: u8 = 0x10;
const SHIFT_R: u8 = 0x11;
const ROT_L: u8 = 0x12;
const CLR: u8 = 0x13;
const LOAD_IO: u8 = 0x14;
const STORE_IO: u8 = 0x15;
const INC: u8 = 0x16;
const DEC: u8 = 0x17;
const ISZ: u8 = 0x18;
const MAX_OPCODE: u8 = 0x18;

/// Read a 16-bit little-endian word from the tape at the given word index.
/// The word index wraps modulo `num_words`.
#[inline]
fn read_word(tape: &[u8], word_idx: u8, num_words: usize) -> i16 {
    let idx = (word_idx as usize % num_words) * 2;
    i16::from_le_bytes([tape[idx], tape[idx + 1]])
}

/// Write a 16-bit little-endian word to the tape at the given word index.
/// The word index wraps modulo `num_words`.
#[inline]
fn write_word(tape: &mut [u8], word_idx: u8, num_words: usize, val: i16) {
    let idx = (word_idx as usize % num_words) * 2;
    let bytes = val.to_le_bytes();
    tape[idx] = bytes[0];
    tape[idx + 1] = bytes[1];
}

impl Substrate for Pdp1 {
    fn execute(tape: &mut [u8], step_limit: usize) -> usize {
        let num_words = tape.len() / 2;
        if num_words == 0 {
            return 0;
        }

        let mut pc: usize = 0; // word index
        let mut ac: i16 = 0;
        let mut io: i16 = 0;
        let mut overflow: bool = false;
        let mut steps: usize = 0;

        while pc < num_words && steps < step_limit {
            steps += 1;

            let byte_offset = pc * 2;
            let opcode = tape[byte_offset] & 0x1F;
            let s = tape[byte_offset + 1]; // address operand (word index)

            match opcode {
                HALT => break,
                ADD => {
                    let mem_val = read_word(tape, s, num_words);
                    let (result, overflowed) = ac.overflowing_add(mem_val);
                    ac = result;
                    if overflowed {
                        overflow = true;
                    }
                }
                SUB => {
                    let mem_val = read_word(tape, s, num_words);
                    let (result, overflowed) = ac.overflowing_sub(mem_val);
                    ac = result;
                    if overflowed {
                        overflow = true;
                    }
                }
                AND => {
                    ac &= read_word(tape, s, num_words);
                }
                IOR => {
                    ac |= read_word(tape, s, num_words);
                }
                XOR => {
                    ac ^= read_word(tape, s, num_words);
                }
                LOAD => {
                    ac = read_word(tape, s, num_words);
                }
                STORE => {
                    write_word(tape, s, num_words, ac);
                }
                LOAD_NEG => {
                    ac = read_word(tape, s, num_words).wrapping_neg();
                }
                SWAP => {
                    let mem_val = read_word(tape, s, num_words);
                    write_word(tape, s, num_words, ac);
                    ac = mem_val;
                }
                JMP => {
                    pc = s as usize % num_words;
                    continue;
                }
                JSR => {
                    io = (pc as i16).wrapping_add(1);
                    pc = s as usize % num_words;
                    continue;
                }
                SKP_Z => {
                    if ac == 0 {
                        pc += 1;
                    }
                }
                SKP_POS => {
                    if ac >= 0 {
                        pc += 1;
                    }
                }
                SKP_NEG => {
                    if ac < 0 {
                        pc += 1;
                    }
                }
                SKP_OVF => {
                    if overflow {
                        pc += 1;
                    }
                    overflow = false;
                }
                SHIFT_L => {
                    let shift = (s & 0x0F) as u32;
                    ac = ac.wrapping_shl(shift);
                }
                SHIFT_R => {
                    let shift = (s & 0x0F) as u32;
                    ac = ac.wrapping_shr(shift);
                }
                ROT_L => {
                    let shift = (s & 0x0F) as u32;
                    ac = (ac as u16).rotate_left(shift) as i16;
                }
                CLR => {
                    ac = 0;
                }
                LOAD_IO => {
                    ac = io;
                }
                STORE_IO => {
                    io = ac;
                }
                INC => {
                    ac = ac.wrapping_add(1);
                }
                DEC => {
                    ac = ac.wrapping_sub(1);
                }
                ISZ => {
                    let mem_val = read_word(tape, s, num_words).wrapping_add(1);
                    write_word(tape, s, num_words, mem_val);
                    if mem_val == 0 {
                        pc += 1;
                    }
                }
                _ => {} // NOP (0x19-0x1F and higher)
            }
            pc += 1;
        }

        steps
    }

    fn is_instruction(byte: u8) -> bool {
        (byte & 0x1F) <= MAX_OPCODE
    }

    fn disassemble(tape: &[u8]) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        let num_words = tape.len() / 2;
        let mut pc = 0;
        while pc < num_words {
            let byte_offset = pc * 2;
            let opcode_byte = tape[byte_offset];
            let operand = tape[byte_offset + 1];
            let opcode = opcode_byte & 0x1F;
            let desc = match opcode {
                HALT => "HALT".to_string(),
                ADD => format!("ADD [{}]", operand),
                SUB => format!("SUB [{}]", operand),
                AND => format!("AND [{}]", operand),
                IOR => format!("IOR [{}]", operand),
                XOR => format!("XOR [{}]", operand),
                LOAD => format!("LOAD [{}]", operand),
                STORE => format!("STORE [{}]", operand),
                LOAD_NEG => format!("LOAD_NEG [{}]", operand),
                SWAP => format!("SWAP [{}]", operand),
                JMP => format!("JMP {}", operand),
                JSR => format!("JSR {}", operand),
                SKP_Z => "SKP_Z".to_string(),
                SKP_POS => "SKP_POS".to_string(),
                SKP_NEG => "SKP_NEG".to_string(),
                SKP_OVF => "SKP_OVF".to_string(),
                SHIFT_L => format!("SHIFT_L {}", operand & 0x0F),
                SHIFT_R => format!("SHIFT_R {}", operand & 0x0F),
                ROT_L => format!("ROT_L {}", operand & 0x0F),
                CLR => "CLR".to_string(),
                LOAD_IO => "LOAD_IO".to_string(),
                STORE_IO => "STORE_IO".to_string(),
                INC => "INC".to_string(),
                DEC => "DEC".to_string(),
                ISZ => format!("ISZ [{}]", operand),
                _ => "NOP".to_string(),
            };
            let _ = writeln!(
                out,
                "{:04X}: {:02X} {:02X}  {}",
                byte_offset, opcode_byte, operand, desc
            );
            pc += 1;
        }
        // Trailing byte if tape length is odd
        if tape.len() % 2 != 0 {
            let i = tape.len() - 1;
            let _ = writeln!(out, "{i:04X}: {:02X}     (trailing)", tape[i]);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a tape from a sequence of (opcode, operand) instruction pairs,
    /// with extra space for data words.
    fn make_tape(instrs: &[(u8, u8)], total_words: usize) -> Vec<u8> {
        let mut tape = vec![0u8; total_words * 2];
        for (i, &(op, arg)) in instrs.iter().enumerate() {
            let off = i * 2;
            if off + 1 < tape.len() {
                tape[off] = op;
                tape[off + 1] = arg;
            }
        }
        tape
    }

    /// Write a 16-bit value at a given word index in the tape.
    fn set_word(tape: &mut [u8], word_idx: usize, val: i16) {
        let off = word_idx * 2;
        let bytes = val.to_le_bytes();
        tape[off] = bytes[0];
        tape[off + 1] = bytes[1];
    }

    /// Read a 16-bit value from a given word index in the tape.
    fn get_word(tape: &[u8], word_idx: usize) -> i16 {
        let off = word_idx * 2;
        i16::from_le_bytes([tape[off], tape[off + 1]])
    }

    #[test]
    fn test_empty_tape() {
        let mut tape: Vec<u8> = vec![];
        let steps = Pdp1::execute(&mut tape, 8192);
        assert_eq!(steps, 0);
    }

    #[test]
    fn test_single_byte_tape() {
        let mut tape = vec![0u8; 1];
        let steps = Pdp1::execute(&mut tape, 8192);
        assert_eq!(steps, 0);
    }

    #[test]
    fn test_halt() {
        // HALT at word 0, then INC at word 1 — should never reach INC
        let mut tape = make_tape(&[(HALT, 0), (INC, 0)], 8);
        let steps = Pdp1::execute(&mut tape, 8192);
        assert_eq!(steps, 1);
    }

    #[test]
    fn test_load() {
        // Put 42 at word 4, LOAD from word 4, STORE to word 5, HALT
        let mut tape = make_tape(&[(LOAD, 4), (STORE, 5), (HALT, 0)], 8);
        set_word(&mut tape, 4, 42);
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 5), 42);
    }

    #[test]
    fn test_store() {
        // LOAD 100 from word 4, STORE to word 5
        let mut tape = make_tape(&[(LOAD, 4), (STORE, 5), (HALT, 0)], 8);
        set_word(&mut tape, 4, 100);
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 5), 100);
    }

    #[test]
    fn test_add() {
        // ac = mem[4]=10, ac += mem[5]=20 => ac=30, store to word 6
        let mut tape = make_tape(&[(LOAD, 4), (ADD, 5), (STORE, 6), (HALT, 0)], 8);
        set_word(&mut tape, 4, 10);
        set_word(&mut tape, 5, 20);
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 6), 30);
    }

    #[test]
    fn test_add_overflow() {
        // ac = i16::MAX, ac += 1 => overflow, ac wraps to i16::MIN
        let mut tape = make_tape(
            &[(LOAD, 5), (ADD, 6), (STORE, 7), (SKP_OVF, 0), (HALT, 0)],
            10,
        );
        set_word(&mut tape, 5, i16::MAX);
        set_word(&mut tape, 6, 1);
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 7), i16::MIN); // wrapped
    }

    #[test]
    fn test_sub() {
        // ac = 50, ac -= 30 => ac=20
        let mut tape = make_tape(&[(LOAD, 4), (SUB, 5), (STORE, 6), (HALT, 0)], 8);
        set_word(&mut tape, 4, 50);
        set_word(&mut tape, 5, 30);
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 6), 20);
    }

    #[test]
    fn test_sub_overflow() {
        // ac = i16::MIN, ac -= 1 => overflow
        let mut tape = make_tape(
            &[(LOAD, 5), (SUB, 6), (STORE, 7), (SKP_OVF, 0), (HALT, 0)],
            10,
        );
        set_word(&mut tape, 5, i16::MIN);
        set_word(&mut tape, 6, 1);
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 7), i16::MAX); // wrapped
    }

    #[test]
    fn test_and() {
        let mut tape = make_tape(&[(LOAD, 4), (AND, 5), (STORE, 6), (HALT, 0)], 8);
        set_word(&mut tape, 4, 0x0F0F);
        set_word(&mut tape, 5, 0x00FF);
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 6), 0x000F);
    }

    #[test]
    fn test_ior() {
        let mut tape = make_tape(&[(LOAD, 4), (IOR, 5), (STORE, 6), (HALT, 0)], 8);
        set_word(&mut tape, 4, 0x0F00);
        set_word(&mut tape, 5, 0x00F0);
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 6), 0x0FF0);
    }

    #[test]
    fn test_xor() {
        let mut tape = make_tape(&[(LOAD, 4), (XOR, 5), (STORE, 6), (HALT, 0)], 8);
        set_word(&mut tape, 4, 0x0FF0);
        set_word(&mut tape, 5, 0x0F0F);
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 6), 0x00FF);
    }

    #[test]
    fn test_load_neg() {
        // ac = -mem[4] = -42
        let mut tape = make_tape(&[(LOAD_NEG, 4), (STORE, 5), (HALT, 0)], 8);
        set_word(&mut tape, 4, 42);
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 5), -42);
    }

    #[test]
    fn test_swap() {
        // ac = mem[4]=10. SWAP mem[5]=20: ac becomes 20, mem[5] becomes 10.
        let mut tape = make_tape(&[(LOAD, 4), (SWAP, 5), (STORE, 6), (HALT, 0)], 8);
        set_word(&mut tape, 4, 10);
        set_word(&mut tape, 5, 20);
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 5), 10); // old ac
        assert_eq!(get_word(&tape, 6), 20); // ac is now the old mem[5]
    }

    #[test]
    fn test_jmp() {
        // JMP to word 3. Word 3 has HALT. Word 1 has INC (should be skipped).
        let mut tape = make_tape(&[(JMP, 3), (INC, 0), (INC, 0), (HALT, 0)], 8);
        let steps = Pdp1::execute(&mut tape, 8192);
        assert_eq!(steps, 2); // JMP + HALT
    }

    #[test]
    fn test_jsr() {
        // JSR to word 3: io = pc+1 = 1, pc = 3. Word 3: LOAD_IO, STORE to word 5, HALT.
        let mut tape = make_tape(
            &[
                (JSR, 3),
                (INC, 0), // skipped
                (INC, 0), // skipped
                (LOAD_IO, 0),
                (STORE, 5),
                (HALT, 0),
            ],
            8,
        );
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 5), 1); // io was pc+1=1
    }

    #[test]
    fn test_skp_z_taken() {
        // ac=0 (initial). SKP_Z skips the HALT at word 1. Word 2 has INC, word 3 STORE, word 4 HALT.
        let mut tape = make_tape(
            &[(SKP_Z, 0), (HALT, 0), (INC, 0), (STORE, 6), (HALT, 0)],
            8,
        );
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 6), 1); // INC ran
    }

    #[test]
    fn test_skp_z_not_taken() {
        // ac != 0. SKP_Z does not skip.
        let mut tape = make_tape(&[(INC, 0), (SKP_Z, 0), (HALT, 0), (INC, 0)], 8);
        let steps = Pdp1::execute(&mut tape, 8192);
        assert_eq!(steps, 3); // INC, SKP_Z (not taken), HALT
    }

    #[test]
    fn test_skp_pos_taken() {
        // ac=0 (which is >= 0). SKP_POS skips HALT.
        let mut tape = make_tape(
            &[(SKP_POS, 0), (HALT, 0), (INC, 0), (STORE, 6), (HALT, 0)],
            8,
        );
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 6), 1);
    }

    #[test]
    fn test_skp_pos_not_taken() {
        // ac < 0. SKP_POS does not skip.
        let mut tape = make_tape(&[(DEC, 0), (SKP_POS, 0), (HALT, 0), (INC, 0)], 8);
        let steps = Pdp1::execute(&mut tape, 8192);
        assert_eq!(steps, 3); // DEC (ac=-1), SKP_POS (not taken), HALT
    }

    #[test]
    fn test_skp_neg_taken() {
        // ac=-1 (after DEC). SKP_NEG skips HALT.
        let mut tape = make_tape(
            &[(DEC, 0), (SKP_NEG, 0), (HALT, 0), (STORE, 6), (HALT, 0)],
            8,
        );
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 6), -1);
    }

    #[test]
    fn test_skp_neg_not_taken() {
        // ac=0. SKP_NEG does not skip.
        let mut tape = make_tape(&[(SKP_NEG, 0), (HALT, 0), (INC, 0)], 8);
        let steps = Pdp1::execute(&mut tape, 8192);
        assert_eq!(steps, 2); // SKP_NEG (not taken), HALT
    }

    #[test]
    fn test_skp_ovf_taken_and_clears() {
        // Force overflow: ac = i16::MAX, ADD 1. SKP_OVF skips HALT. Then check overflow is clear.
        let mut tape = make_tape(
            &[
                (LOAD, 8),   // ac = i16::MAX
                (ADD, 9),    // ac += 1 => overflow
                (SKP_OVF, 0), // skip (overflow set), clears overflow
                (HALT, 0),   // skipped
                (SKP_OVF, 0), // not taken (overflow cleared)
                (HALT, 0),   // reached
            ],
            10,
        );
        set_word(&mut tape, 8, i16::MAX);
        set_word(&mut tape, 9, 1);
        let steps = Pdp1::execute(&mut tape, 8192);
        // w0=LOAD, w1=ADD, w2=SKP_OVF(skip+clear), w3=HALT(skipped),
        // w4=SKP_OVF(not taken, overflow already cleared), w5=HALT
        assert_eq!(steps, 5);
    }

    #[test]
    fn test_skp_ovf_not_taken_when_no_overflow() {
        // No overflow happened. SKP_OVF does not skip.
        let mut tape = make_tape(&[(SKP_OVF, 0), (HALT, 0), (INC, 0)], 8);
        let steps = Pdp1::execute(&mut tape, 8192);
        assert_eq!(steps, 2); // SKP_OVF (not taken), HALT
    }

    #[test]
    fn test_shift_l() {
        // ac = 1, shift left by 3 => 8
        let mut tape = make_tape(&[(LOAD, 4), (SHIFT_L, 3), (STORE, 5), (HALT, 0)], 8);
        set_word(&mut tape, 4, 1);
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 5), 8);
    }

    #[test]
    fn test_shift_r() {
        // ac = -16, arithmetic shift right by 2 => -4
        let mut tape = make_tape(&[(LOAD, 4), (SHIFT_R, 2), (STORE, 5), (HALT, 0)], 8);
        set_word(&mut tape, 4, -16);
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 5), -4);
    }

    #[test]
    fn test_rot_l() {
        // ac = 0x8001 as u16 = -32767 as i16. Rotate left by 1 => 0x0003.
        let mut tape = make_tape(&[(LOAD, 4), (ROT_L, 1), (STORE, 5), (HALT, 0)], 8);
        set_word(&mut tape, 4, 0x8001_u16 as i16);
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 5) as u16, 0x0003);
    }

    #[test]
    fn test_clr() {
        // ac = 42, CLR => ac = 0
        let mut tape = make_tape(&[(LOAD, 4), (CLR, 0), (STORE, 5), (HALT, 0)], 8);
        set_word(&mut tape, 4, 42);
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 5), 0);
    }

    #[test]
    fn test_load_io_store_io() {
        // STORE_IO saves ac to io. Load something else. LOAD_IO retrieves it.
        let mut tape = make_tape(
            &[
                (LOAD, 6),     // ac = 99
                (STORE_IO, 0), // io = 99
                (CLR, 0),      // ac = 0
                (LOAD_IO, 0),  // ac = io = 99
                (STORE, 7),    // mem[7] = 99
                (HALT, 0),
            ],
            8,
        );
        set_word(&mut tape, 6, 99);
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 7), 99);
    }

    #[test]
    fn test_inc() {
        // ac=0, INC => ac=1
        let mut tape = make_tape(&[(INC, 0), (STORE, 4), (HALT, 0)], 8);
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 4), 1);
    }

    #[test]
    fn test_dec() {
        // ac=0, DEC => ac=-1
        let mut tape = make_tape(&[(DEC, 0), (STORE, 4), (HALT, 0)], 8);
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 4), -1);
    }

    #[test]
    fn test_isz_no_skip() {
        // mem[4] = 5. ISZ: mem[4] becomes 6 (not zero), no skip.
        let mut tape = make_tape(&[(ISZ, 4), (HALT, 0), (INC, 0)], 8);
        set_word(&mut tape, 4, 5);
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 4), 6);
    }

    #[test]
    fn test_isz_skip() {
        // mem[4] = -1. ISZ: mem[4] becomes 0, skip HALT.
        let mut tape = make_tape(
            &[(ISZ, 4), (HALT, 0), (INC, 0), (STORE, 5), (HALT, 0)],
            8,
        );
        set_word(&mut tape, 4, -1);
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 4), 0);
        assert_eq!(get_word(&tape, 5), 1); // INC ran because HALT was skipped
    }

    #[test]
    fn test_nop_bytes() {
        // Opcodes 0x19-0x1F are NOPs. They should just advance pc.
        let mut tape = make_tape(&[(0x19, 0), (0x1F, 0), (INC, 0), (STORE, 5), (HALT, 0)], 8);
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 5), 1); // only one INC
    }

    #[test]
    fn test_step_limit() {
        // Infinite loop: JMP 0
        let mut tape = make_tape(&[(JMP, 0)], 4);
        let steps = Pdp1::execute(&mut tape, 100);
        assert_eq!(steps, 100);
    }

    #[test]
    fn test_address_wrapping() {
        // In a tape with 8 words, address 10 wraps to 10%8 = 2.
        let mut tape = make_tape(&[(LOAD, 10), (STORE, 5), (HALT, 0)], 8);
        // Word 2 (which is where address 10 wraps to) = word 2 = the HALT instruction.
        // Let's set word 2 explicitly.
        set_word(&mut tape, 2, 777);
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 5), 777);
    }

    #[test]
    fn test_is_instruction() {
        for op in 0..=MAX_OPCODE {
            assert!(Pdp1::is_instruction(op), "opcode {op} should be instruction");
        }
        // Opcodes with higher bits set but low 5 bits in range should also be true
        assert!(Pdp1::is_instruction(0x20 | LOAD)); // 0x26
        assert!(Pdp1::is_instruction(0xE0 | ISZ)); // 0xF8

        // Low 5 bits > MAX_OPCODE should be false
        assert!(!Pdp1::is_instruction(0x19));
        assert!(!Pdp1::is_instruction(0x1F));
    }

    #[test]
    fn test_disassemble() {
        let tape = make_tape(&[(LOAD, 4), (ADD, 5), (STORE, 6), (HALT, 0)], 8);
        let disasm = Pdp1::disassemble(&tape);
        assert!(disasm.contains("LOAD"));
        assert!(disasm.contains("ADD"));
        assert!(disasm.contains("STORE"));
        assert!(disasm.contains("HALT"));
    }

    #[test]
    fn test_disassemble_trailing_byte() {
        // Odd-length tape should show trailing byte
        let tape = vec![LOAD, 4, HALT, 0, 0xFF];
        let disasm = Pdp1::disassemble(&tape);
        assert!(disasm.contains("trailing"));
    }

    #[test]
    fn test_jsr_return_pattern() {
        // JSR saves return address in io. Subroutine can JMP back via LOAD_IO + use as address.
        // w0: JSR 3 (io=1, pc=3)
        // w1: STORE 7, w2: HALT (return point and after)
        // w3: INC (subroutine body)
        // w4: LOAD_IO => ac = 1
        // w5: actually we need to JMP to the return address stored in io.
        // Since JMP takes an operand from the instruction, not ac, we need SWAP.
        // Simpler test: just verify io holds the right return address.
        let mut tape = make_tape(
            &[
                (JSR, 3),      // w0: io = 1, pc = 3
                (STORE, 7),    // w1: (return target)
                (HALT, 0),     // w2:
                (LOAD_IO, 0),  // w3: ac = io = 1
                (STORE, 7),    // w4: mem[7] = 1
                (HALT, 0),     // w5:
            ],
            8,
        );
        Pdp1::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 7), 1); // io was set to pc+1=1
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
            let steps = Pdp1::execute(&mut tape, 8192);
            prop_assert!(steps <= 8192);
        }

        #[test]
        fn random_programs_respect_step_limit(
            tape_data in prop::collection::vec(any::<u8>(), 1..256),
            limit in 1usize..1000
        ) {
            let mut tape = tape_data;
            let steps = Pdp1::execute(&mut tape, limit);
            prop_assert!(steps <= limit);
        }

        #[test]
        fn output_tape_same_length(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let original_len = tape_data.len();
            let mut tape = tape_data;
            Pdp1::execute(&mut tape, 8192);
            prop_assert_eq!(tape.len(), original_len);
        }
    }
}
