use crate::substrate::Substrate;

/// The IAS Machine (1952) substrate — the canonical von Neumann architecture.
///
/// The original IAS machine used 40-bit words with two 20-bit instructions per
/// word, an 8-bit opcode + 12-bit address, a single accumulator (AC) and a
/// multiplier-quotient register (MQ).
///
/// This adaptation maps that design onto a byte tape using 2-byte instructions
/// (1 byte opcode + 1 byte address). Instructions are sequential byte pairs
/// rather than word-packed, simplifying the model while preserving the IAS
/// flavor.
///
/// State:
/// - `pc`: program counter (byte-pair index), starts at 0
/// - `ac`: accumulator (i16, wrapping arithmetic)
/// - `mq`: multiplier-quotient register (i16)
///
/// Memory is 2-byte words (i16, little-endian). Address S from the operand
/// byte indexes words: `read_word(tape, S % num_words)` reads an i16 at byte
/// offset `(S % num_words) * 2`.
pub struct Ias;

// Opcodes (low 5 bits of first byte)
const HALT: u8 = 0x00;
const LOAD: u8 = 0x01;
const LOAD_NEG: u8 = 0x02;
const LOAD_ABS: u8 = 0x03;
const LOAD_MQ: u8 = 0x04;
const LOAD_MQ_M: u8 = 0x05;
const STORE: u8 = 0x06;
const ADD: u8 = 0x07;
const SUB: u8 = 0x08;
const MUL: u8 = 0x09;
const DIV: u8 = 0x0A;
const JMP: u8 = 0x0B;
const JMP_POS: u8 = 0x0C;
const SHIFT_L: u8 = 0x0D;
const SHIFT_R: u8 = 0x0E;
const STORE_ADDR: u8 = 0x0F;
const AND: u8 = 0x10;
const OR: u8 = 0x11;
const XOR: u8 = 0x12;

const MAX_OPCODE: u8 = XOR;

/// Read a little-endian i16 word at the given word index.
#[inline]
fn read_word(tape: &[u8], num_words: usize, addr: u8) -> i16 {
    let idx = (addr as usize % num_words) * 2;
    i16::from_le_bytes([tape[idx], tape[idx + 1]])
}

/// Write a little-endian i16 word at the given word index.
#[inline]
fn write_word(tape: &mut [u8], num_words: usize, addr: u8, val: i16) {
    let idx = (addr as usize % num_words) * 2;
    let bytes = val.to_le_bytes();
    tape[idx] = bytes[0];
    tape[idx + 1] = bytes[1];
}

impl Substrate for Ias {
    fn execute(tape: &mut [u8], step_limit: usize) -> usize {
        let len = tape.len();
        if len < 2 {
            return 0;
        }

        let num_words = len / 2;
        let num_instr_slots = num_words; // each instruction is one word (2 bytes)

        let mut pc: usize = 0;
        let mut ac: i16 = 0;
        let mut mq: i16 = 0;
        let mut steps: usize = 0;

        while pc < num_instr_slots && steps < step_limit {
            steps += 1;
            let byte_offset = pc * 2;
            let opcode = tape[byte_offset] & 0x1F;
            let s = tape[byte_offset + 1];

            match opcode {
                HALT => break,
                LOAD => {
                    ac = read_word(tape, num_words, s);
                }
                LOAD_NEG => {
                    ac = read_word(tape, num_words, s).wrapping_neg();
                }
                LOAD_ABS => {
                    let val = read_word(tape, num_words, s);
                    ac = val.wrapping_abs();
                }
                LOAD_MQ => {
                    ac = mq;
                }
                LOAD_MQ_M => {
                    mq = read_word(tape, num_words, s);
                    ac = mq;
                }
                STORE => {
                    write_word(tape, num_words, s, ac);
                }
                ADD => {
                    ac = ac.wrapping_add(read_word(tape, num_words, s));
                }
                SUB => {
                    ac = ac.wrapping_sub(read_word(tape, num_words, s));
                }
                MUL => {
                    let val = read_word(tape, num_words, s);
                    let product = (mq as i32).wrapping_mul(val as i32);
                    mq = product as i16; // low 16 bits
                    ac = (product >> 16) as i16; // high 16 bits
                }
                DIV => {
                    let val = read_word(tape, num_words, s);
                    if val == 0 {
                        break; // division by zero halts
                    }
                    mq = ac.wrapping_div(val);
                    ac = ac.wrapping_rem(val);
                }
                JMP => {
                    pc = s as usize;
                    continue;
                }
                JMP_POS => {
                    if ac >= 0 {
                        pc = s as usize;
                        continue;
                    }
                }
                SHIFT_L => {
                    let shift = (s & 0x0F) as u32;
                    ac = ac.wrapping_shl(shift);
                }
                SHIFT_R => {
                    let shift = (s & 0x0F) as u32;
                    ac = ac.wrapping_shr(shift); // arithmetic shift for i16
                }
                STORE_ADDR => {
                    let idx = (s as usize % num_words) * 2;
                    tape[idx] = (ac & 0xFF) as u8;
                }
                AND => {
                    ac &= read_word(tape, num_words, s);
                }
                OR => {
                    ac |= read_word(tape, num_words, s);
                }
                XOR => {
                    ac ^= read_word(tape, num_words, s);
                }
                _ => {} // NOP
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
        let len = tape.len();
        if len < 2 {
            return out;
        }
        let num_words = len / 2;
        for slot in 0..num_words {
            let byte_offset = slot * 2;
            let raw_op = tape[byte_offset];
            let opcode = raw_op & 0x1F;
            let s = tape[byte_offset + 1];
            let desc = match opcode {
                HALT => "HALT".to_string(),
                LOAD => format!("LOAD {s}"),
                LOAD_NEG => format!("LOAD_NEG {s}"),
                LOAD_ABS => format!("LOAD_ABS {s}"),
                LOAD_MQ => "LOAD_MQ".to_string(),
                LOAD_MQ_M => format!("LOAD_MQ_M {s}"),
                STORE => format!("STORE {s}"),
                ADD => format!("ADD {s}"),
                SUB => format!("SUB {s}"),
                MUL => format!("MUL {s}"),
                DIV => format!("DIV {s}"),
                JMP => format!("JMP {s}"),
                JMP_POS => format!("JMP_POS {s}"),
                SHIFT_L => format!("SHIFT_L {}", s & 0x0F),
                SHIFT_R => format!("SHIFT_R {}", s & 0x0F),
                STORE_ADDR => format!("STORE_ADDR {s}"),
                AND => format!("AND {s}"),
                OR => format!("OR {s}"),
                XOR => format!("XOR {s}"),
                _ => "NOP".to_string(),
            };
            let _ = writeln!(
                out,
                "{byte_offset:04X}: {:02X} {:02X}  {desc}",
                raw_op, s
            );
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a tape of `size` bytes, placing `program` at the start.
    /// `program` is a flat sequence of (opcode, operand) byte pairs.
    fn make_tape(program: &[u8], size: usize) -> Vec<u8> {
        let mut tape = vec![0u8; size];
        for (i, &b) in program.iter().enumerate() {
            if i < size {
                tape[i] = b;
            }
        }
        tape
    }

    /// Helper: write an i16 word at word-address `addr` in a tape.
    fn set_word(tape: &mut [u8], addr: usize, val: i16) {
        let idx = addr * 2;
        let bytes = val.to_le_bytes();
        tape[idx] = bytes[0];
        tape[idx + 1] = bytes[1];
    }

    /// Helper: read an i16 word at word-address `addr` in a tape.
    fn get_word(tape: &[u8], addr: usize) -> i16 {
        let idx = addr * 2;
        i16::from_le_bytes([tape[idx], tape[idx + 1]])
    }

    #[test]
    fn test_empty_tape() {
        let mut tape: Vec<u8> = vec![];
        let steps = Ias::execute(&mut tape, 8192);
        assert_eq!(steps, 0);
    }

    #[test]
    fn test_single_byte_tape() {
        let mut tape = vec![0x01];
        let steps = Ias::execute(&mut tape, 8192);
        assert_eq!(steps, 0);
    }

    #[test]
    fn test_halt() {
        // Slot 0: HALT, Slot 1: LOAD (should never run)
        let mut tape = make_tape(&[HALT, 0x00, LOAD, 0x05], 128);
        let steps = Ias::execute(&mut tape, 8192);
        assert_eq!(steps, 1);
    }

    #[test]
    fn test_load() {
        // Put value 42 at word address 10. LOAD 10 -> ac=42. STORE 11 -> word[11]=42.
        let mut tape = make_tape(
            &[LOAD, 10, STORE, 11, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, 42);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 11), 42);
    }

    #[test]
    fn test_load_neg() {
        // Put value 42 at word address 10. LOAD_NEG 10 -> ac=-42. STORE 11.
        let mut tape = make_tape(
            &[LOAD_NEG, 10, STORE, 11, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, 42);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 11), -42);
    }

    #[test]
    fn test_load_abs() {
        // Put value -7 at word address 10. LOAD_ABS 10 -> ac=7. STORE 11.
        let mut tape = make_tape(
            &[LOAD_ABS, 10, STORE, 11, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, -7);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 11), 7);
    }

    #[test]
    fn test_load_mq() {
        // LOAD_MQ_M from word 10 (value 99) -> mq=99, ac=99.
        // Then LOAD 11 (value 0) -> ac=0.
        // Then LOAD_MQ -> ac=mq=99. STORE 12.
        let mut tape = make_tape(
            &[LOAD_MQ_M, 10, LOAD, 11, LOAD_MQ, 0x00, STORE, 12, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, 99);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 12), 99);
    }

    #[test]
    fn test_load_mq_m() {
        // LOAD_MQ_M from word 10 (value 55) -> mq=55, ac=55. STORE 11.
        let mut tape = make_tape(
            &[LOAD_MQ_M, 10, STORE, 11, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, 55);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 11), 55);
    }

    #[test]
    fn test_store() {
        // LOAD 10 (value 123). STORE 11. Check word 11.
        let mut tape = make_tape(
            &[LOAD, 10, STORE, 11, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, 123);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 11), 123);
    }

    #[test]
    fn test_add() {
        // LOAD 10 (value 30). ADD 11 (value 12). STORE 12.
        let mut tape = make_tape(
            &[LOAD, 10, ADD, 11, STORE, 12, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, 30);
        set_word(&mut tape, 11, 12);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 12), 42);
    }

    #[test]
    fn test_add_wrapping() {
        // LOAD 10 (i16::MAX). ADD 11 (1). Wraps to i16::MIN.
        let mut tape = make_tape(
            &[LOAD, 10, ADD, 11, STORE, 12, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, i16::MAX);
        set_word(&mut tape, 11, 1);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 12), i16::MIN);
    }

    #[test]
    fn test_sub() {
        // LOAD 10 (value 50). SUB 11 (value 8). STORE 12.
        let mut tape = make_tape(
            &[LOAD, 10, SUB, 11, STORE, 12, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, 50);
        set_word(&mut tape, 11, 8);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 12), 42);
    }

    #[test]
    fn test_mul() {
        // LOAD_MQ_M 10 (value 7). MUL 11 (value 6). mq = 42, ac = 0 (small product).
        // Then LOAD_MQ -> ac=42. STORE 12.
        let mut tape = make_tape(
            &[LOAD_MQ_M, 10, MUL, 11, LOAD_MQ, 0x00, STORE, 12, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, 7);
        set_word(&mut tape, 11, 6);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 12), 42);
    }

    #[test]
    fn test_mul_high_bits() {
        // LOAD_MQ_M 10 (value 1000). MUL 11 (value 1000). Product = 1_000_000.
        // 1_000_000 = 0x000F_4240. mq = 0x4240, ac = 0x000F.
        let mut tape = make_tape(
            &[LOAD_MQ_M, 10, MUL, 11, STORE, 12, LOAD_MQ, 0x00, STORE, 13, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, 1000);
        set_word(&mut tape, 11, 1000);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 12), 0x000F); // high bits in ac
        assert_eq!(get_word(&tape, 13), 0x4240u16 as i16); // low bits in mq
    }

    #[test]
    fn test_div() {
        // LOAD 10 (value 42). DIV 11 (value 5). mq=42/5=8, ac=42%5=2.
        // STORE 12 (ac=2). LOAD_MQ. STORE 13 (mq=8).
        let mut tape = make_tape(
            &[LOAD, 10, DIV, 11, STORE, 12, LOAD_MQ, 0x00, STORE, 13, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, 42);
        set_word(&mut tape, 11, 5);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 12), 2);  // remainder
        assert_eq!(get_word(&tape, 13), 8);  // quotient
    }

    #[test]
    fn test_div_by_zero_halts() {
        // LOAD 10 (value 42). DIV 11 (value 0). Should halt immediately.
        // The STORE should never execute.
        let mut tape = make_tape(
            &[LOAD, 10, DIV, 11, STORE, 12, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, 42);
        set_word(&mut tape, 11, 0);
        let steps = Ias::execute(&mut tape, 8192);
        assert_eq!(steps, 2); // LOAD + DIV (halts on DIV)
        assert_eq!(get_word(&tape, 12), 0); // STORE never ran
    }

    #[test]
    fn test_jmp() {
        // Slot 0: JMP 3. Slot 1: HALT (skipped). Slot 2: HALT (skipped).
        // Slot 3: LOAD 10 (value 77). Slot 4: STORE 11. Slot 5: HALT.
        let mut tape = make_tape(
            &[
                JMP, 3,
                HALT, 0x00,
                HALT, 0x00,
                LOAD, 10,
                STORE, 11,
                HALT, 0x00,
            ],
            128,
        );
        set_word(&mut tape, 10, 77);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 11), 77);
    }

    #[test]
    fn test_jmp_pos_taken() {
        // LOAD 10 (value 5, positive). JMP_POS 4. Slot 2: HALT (skipped).
        // Slot 3: HALT (skipped). Slot 4: STORE 11. Slot 5: HALT.
        let mut tape = make_tape(
            &[
                LOAD, 10,
                JMP_POS, 4,
                HALT, 0x00,
                HALT, 0x00,
                STORE, 11,
                HALT, 0x00,
            ],
            128,
        );
        set_word(&mut tape, 10, 5);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 11), 5);
    }

    #[test]
    fn test_jmp_pos_not_taken() {
        // LOAD 10 (value -1, negative). JMP_POS 4 (not taken). STORE 11. HALT.
        let mut tape = make_tape(
            &[
                LOAD, 10,
                JMP_POS, 4,
                STORE, 11,
                HALT, 0x00,
            ],
            128,
        );
        set_word(&mut tape, 10, -1);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 11), -1);
    }

    #[test]
    fn test_jmp_pos_zero_taken() {
        // ac=0 (>= 0), so JMP_POS should be taken.
        // Slot 0: JMP_POS 2. Slot 1: HALT (skipped). Slot 2: STORE 10. Slot 3: HALT.
        let mut tape = make_tape(
            &[
                JMP_POS, 2,
                HALT, 0x00,
                STORE, 10,
                HALT, 0x00,
            ],
            128,
        );
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 10), 0); // ac was 0, stored 0
    }

    #[test]
    fn test_shift_left() {
        // LOAD 10 (value 1). SHIFT_L 3. ac = 1 << 3 = 8. STORE 11.
        let mut tape = make_tape(
            &[LOAD, 10, SHIFT_L, 3, STORE, 11, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, 1);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 11), 8);
    }

    #[test]
    fn test_shift_right() {
        // LOAD 10 (value -8). SHIFT_R 2. ac = -8 >> 2 = -2 (arithmetic shift). STORE 11.
        let mut tape = make_tape(
            &[LOAD, 10, SHIFT_R, 2, STORE, 11, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, -8);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 11), -2);
    }

    #[test]
    fn test_shift_right_positive() {
        // LOAD 10 (value 16). SHIFT_R 2. ac = 16 >> 2 = 4. STORE 11.
        let mut tape = make_tape(
            &[LOAD, 10, SHIFT_R, 2, STORE, 11, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, 16);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 11), 4);
    }

    #[test]
    fn test_shift_uses_low_4_bits() {
        // Operand is 0xF3 but only low 4 bits (3) are used for shift amount.
        let mut tape = make_tape(
            &[LOAD, 10, SHIFT_L, 0xF3, STORE, 11, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, 1);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 11), 8); // 1 << 3 = 8
    }

    #[test]
    fn test_store_addr() {
        // LOAD 10 (value 0x1234). STORE_ADDR 11. Only low byte (0x34) is stored.
        let mut tape = make_tape(
            &[LOAD, 10, STORE_ADDR, 11, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, 0x1234);
        Ias::execute(&mut tape, 8192);
        // STORE_ADDR writes only the low byte of ac to the low byte of the target word.
        assert_eq!(tape[11 * 2], 0x34);
    }

    #[test]
    fn test_and() {
        // LOAD 10 (value 0x0F0F). AND 11 (value 0x00FF). STORE 12.
        let mut tape = make_tape(
            &[LOAD, 10, AND, 11, STORE, 12, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, 0x0F0F);
        set_word(&mut tape, 11, 0x00FF);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 12), 0x000F);
    }

    #[test]
    fn test_or() {
        // LOAD 10 (value 0x0F00). OR 11 (value 0x00F0). STORE 12.
        let mut tape = make_tape(
            &[LOAD, 10, OR, 11, STORE, 12, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, 0x0F00);
        set_word(&mut tape, 11, 0x00F0);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 12), 0x0FF0);
    }

    #[test]
    fn test_xor() {
        // LOAD 10 (value 0xFF00). XOR 11 (value 0x0FF0). STORE 12.
        let mut tape = make_tape(
            &[LOAD, 10, XOR, 11, STORE, 12, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, 0x7F00u16 as i16);
        set_word(&mut tape, 11, 0x0FF0);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 12), 0x70F0);
    }

    #[test]
    fn test_nop_bytes() {
        // Opcodes 0x13-0x1F are NOP. Just advances pc.
        let mut tape = make_tape(
            &[0x13, 0x00, 0x14, 0x00, 0x1F, 0x00, LOAD, 10, STORE, 11, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, 77);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 11), 77);
    }

    #[test]
    fn test_high_bits_ignored_for_opcode() {
        // Byte 0xE1 has low 5 bits = 0x01 = LOAD. Should work as LOAD.
        let mut tape = make_tape(
            &[0xE1, 10, STORE, 11, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, 33);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 11), 33);
    }

    #[test]
    fn test_step_limit() {
        // Infinite loop: JMP 0 (jump to self forever)
        let mut tape = make_tape(&[JMP, 0x00], 128);
        let steps = Ias::execute(&mut tape, 100);
        assert_eq!(steps, 100);
    }

    #[test]
    fn test_address_wraps() {
        // Tape has 64 words (128 bytes). Address 200 wraps to 200 % 64 = 8.
        let mut tape = make_tape(
            &[LOAD, 200, STORE, 11, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 200 % 64, 999);
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 11), 999);
    }

    #[test]
    fn test_disassemble() {
        let tape = vec![LOAD, 10, ADD, 5, HALT, 0x00];
        let asm = Ias::disassemble(&tape);
        assert!(asm.contains("LOAD 10"));
        assert!(asm.contains("ADD 5"));
        assert!(asm.contains("HALT"));
    }

    #[test]
    fn test_disassemble_empty() {
        let tape: Vec<u8> = vec![];
        let asm = Ias::disassemble(&tape);
        assert!(asm.is_empty());
    }

    #[test]
    fn test_disassemble_single_byte() {
        let tape = vec![0x01];
        let asm = Ias::disassemble(&tape);
        assert!(asm.is_empty());
    }

    #[test]
    fn test_is_instruction() {
        assert!(Ias::is_instruction(HALT));
        assert!(Ias::is_instruction(LOAD));
        assert!(Ias::is_instruction(XOR));
        assert!(!Ias::is_instruction(0x13));
        assert!(!Ias::is_instruction(0xFF));
        // High bits should not matter — only low 5 bits checked.
        assert!(Ias::is_instruction(0xE1)); // low 5 bits = 0x01 = LOAD
        assert!(!Ias::is_instruction(0x33)); // low 5 bits = 0x13 = NOP
    }

    #[test]
    fn test_program_counter_increments() {
        // Verify that each instruction advances pc by 1 (one slot = 2 bytes).
        // LOAD 10, ADD 10, ADD 10, STORE 11, HALT.
        let mut tape = make_tape(
            &[LOAD, 10, ADD, 10, ADD, 10, STORE, 11, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, 5);
        let steps = Ias::execute(&mut tape, 8192);
        assert_eq!(steps, 5); // 4 instructions + HALT
        assert_eq!(get_word(&tape, 11), 15); // 5 + 5 + 5
    }

    #[test]
    fn test_self_modifying_code() {
        // Use STORE to modify a data word that a later instruction reads.
        // Slot 0: LOAD 10 (value 42).
        // Slot 1: STORE 20 (writes ac=42 to word 20).
        // Slot 2: LOAD 20 (now reads 42 instead of original 0).
        // Slot 3: STORE 11.
        // Slot 4: HALT.
        let mut tape = make_tape(
            &[LOAD, 10, STORE, 20, LOAD, 20, STORE, 11, HALT, 0x00],
            128,
        );
        set_word(&mut tape, 10, 42);
        // word 20 starts as 0, but STORE 20 will write 42 there before LOAD 20 reads it.
        Ias::execute(&mut tape, 8192);
        assert_eq!(get_word(&tape, 11), 42);
        assert_eq!(get_word(&tape, 20), 42);
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
            let steps = Ias::execute(&mut tape, 8192);
            prop_assert!(steps <= 8192);
        }

        #[test]
        fn random_programs_respect_step_limit(
            tape_data in prop::collection::vec(any::<u8>(), 1..256),
            limit in 1usize..1000
        ) {
            let mut tape = tape_data;
            let steps = Ias::execute(&mut tape, limit);
            prop_assert!(steps <= limit);
        }

        #[test]
        fn output_tape_same_length(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let original_len = tape_data.len();
            let mut tape = tape_data;
            Ias::execute(&mut tape, 8192);
            prop_assert_eq!(tape.len(), original_len);
        }
    }
}
