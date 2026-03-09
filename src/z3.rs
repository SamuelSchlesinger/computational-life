use crate::substrate::Substrate;

/// The Zuse Z3 (1941) substrate — arguably the world's first programmable
/// digital computer.
///
/// The defining characteristic of the Z3 is the **complete absence of
/// conditional branching**. Programs are purely sequential: the PC always
/// advances by one word, with no jumps, skips, or branches. The only way
/// execution can "loop" is through self-modification (STORE instructions
/// rewriting future instruction bytes on the tape) or by hitting the step
/// limit.
///
/// The real Z3 used 22-bit floating-point words (1 sign + 7 exponent +
/// 14 mantissa) and had 64 words of memory. We simplify to i16 (2-byte
/// little-endian) arithmetic on the byte tape.
///
/// State:
/// - `pc`: program counter (word index), starts at 0
/// - `r1`: register 1 (i16, wrapping), starts at 0
/// - `r2`: register 2 (i16, wrapping), starts at 0
/// - Number of words = tape.len() / 2
///
/// Instruction encoding: 2-byte words (little-endian). The low 4 bits of the
/// first byte select the opcode. The second byte is the operand `S` (a word
/// address, used by LOAD/STORE/COPY_FWD).
pub struct Z3;

// Opcodes (low 4 bits of first byte)
const HALT: u8 = 0x0;
const LOAD1: u8 = 0x1;
const LOAD2: u8 = 0x2;
const STORE1: u8 = 0x3;
const STORE2: u8 = 0x4;
const ADD: u8 = 0x5;
const SUB: u8 = 0x6;
const MUL: u8 = 0x7;
const DIV: u8 = 0x8;
const SQRT: u8 = 0x9;
const NEG: u8 = 0xA;
const ABS: u8 = 0xB;
const MOD: u8 = 0xC;
const SWAP: u8 = 0xD;
const COPY_FWD: u8 = 0xE;
const NOP: u8 = 0xF;

/// Read word at word-index `idx` (wrapping) from the tape.
#[inline]
fn read_word(tape: &[u8], idx: usize, num_words: usize) -> i16 {
    let w = idx % num_words;
    let lo = tape[w * 2] as u16;
    let hi = tape[w * 2 + 1] as u16;
    (lo | (hi << 8)) as i16
}

/// Write word at word-index `idx` (wrapping) into the tape.
#[inline]
fn write_word(tape: &mut [u8], idx: usize, num_words: usize, val: i16) {
    let w = idx % num_words;
    let bytes = (val as u16).to_le_bytes();
    tape[w * 2] = bytes[0];
    tape[w * 2 + 1] = bytes[1];
}

/// Integer square root of a non-negative value.
#[inline]
fn isqrt(val: i16) -> i16 {
    let abs_val = (val as i32).unsigned_abs();
    (abs_val as f64).sqrt() as i16
}

impl Substrate for Z3 {
    fn execute(tape: &mut [u8], step_limit: usize) -> usize {
        let num_words = tape.len() / 2;
        if num_words == 0 {
            return 0;
        }

        let mut pc: usize = 0;
        let mut r1: i16 = 0;
        let mut r2: i16 = 0;
        let mut steps: usize = 0;

        while pc < num_words && steps < step_limit {
            steps += 1;

            let opcode = tape[pc * 2] & 0x0F;
            let operand = tape[pc * 2 + 1] as usize;

            match opcode {
                HALT => break,
                LOAD1 => {
                    r1 = read_word(tape, operand, num_words);
                }
                LOAD2 => {
                    r2 = read_word(tape, operand, num_words);
                }
                STORE1 => {
                    write_word(tape, operand, num_words, r1);
                }
                STORE2 => {
                    write_word(tape, operand, num_words, r2);
                }
                ADD => {
                    r1 = r1.wrapping_add(r2);
                }
                SUB => {
                    r1 = r1.wrapping_sub(r2);
                }
                MUL => {
                    r1 = r1.wrapping_mul(r2);
                }
                DIV => {
                    if r2 != 0 {
                        r1 = r1.wrapping_div(r2);
                    } else {
                        r1 = 0;
                    }
                }
                SQRT => {
                    r1 = isqrt(r1.wrapping_abs());
                }
                NEG => {
                    r1 = r1.wrapping_neg();
                }
                ABS => {
                    r1 = r1.wrapping_abs();
                }
                MOD => {
                    if r2 != 0 {
                        r1 = r1.wrapping_rem(r2);
                    } else {
                        r1 = 0;
                    }
                }
                SWAP => {
                    core::mem::swap(&mut r1, &mut r2);
                }
                COPY_FWD => {
                    let src = (operand + 1) % num_words;
                    let val = read_word(tape, src, num_words);
                    write_word(tape, operand, num_words, val);
                }
                NOP | _ => {
                    // NOP (0xF) and any other value — do nothing
                }
            }

            // The Z3 has NO branching. PC always advances by 1.
            pc += 1;
        }

        steps
    }

    fn is_instruction(byte: u8) -> bool {
        (byte & 0x0F) <= COPY_FWD
    }

    fn disassemble(tape: &[u8]) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        let num_words = tape.len() / 2;
        for w in 0..num_words {
            let b0 = tape[w * 2];
            let b1 = tape[w * 2 + 1];
            let opcode = b0 & 0x0F;
            let s = b1 as usize;
            let desc = match opcode {
                HALT => "HALT".to_string(),
                LOAD1 => format!("LOAD1 [{s}]"),
                LOAD2 => format!("LOAD2 [{s}]"),
                STORE1 => format!("STORE1 [{s}]"),
                STORE2 => format!("STORE2 [{s}]"),
                ADD => "ADD".to_string(),
                SUB => "SUB".to_string(),
                MUL => "MUL".to_string(),
                DIV => "DIV".to_string(),
                SQRT => "SQRT".to_string(),
                NEG => "NEG".to_string(),
                ABS => "ABS".to_string(),
                MOD => "MOD".to_string(),
                SWAP => "SWAP".to_string(),
                COPY_FWD => {
                    let src = (s + 1) % num_words.max(1);
                    format!("COPY_FWD [{s}] <- [{src}]")
                }
                _ => "NOP".to_string(),
            };
            let _ = writeln!(out, "{:04X}: {:02X} {:02X}  {}", w * 2, b0, b1, desc);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a tape of `size` bytes (must be even), placing encoded 2-byte
    /// instructions starting at byte 0.
    fn make_tape(words: &[(u8, u8)], size: usize) -> Vec<u8> {
        let mut tape = vec![0u8; size];
        for (i, &(lo, hi)) in words.iter().enumerate() {
            let off = i * 2;
            if off + 1 < size {
                tape[off] = lo;
                tape[off + 1] = hi;
            }
        }
        tape
    }

    /// Encode an instruction word: low 4 bits = opcode, second byte = operand.
    fn instr(opcode: u8, operand: u8) -> (u8, u8) {
        (opcode & 0x0F, operand)
    }

    // ─── Basic execution tests ──────────────────────────────────────────

    #[test]
    fn test_empty_tape() {
        let mut tape: Vec<u8> = vec![];
        let steps = Z3::execute(&mut tape, 8192);
        assert_eq!(steps, 0);
    }

    #[test]
    fn test_single_byte_tape() {
        let mut tape = vec![0x05]; // odd length => 0 words
        let steps = Z3::execute(&mut tape, 8192);
        assert_eq!(steps, 0);
    }

    #[test]
    fn test_halt() {
        let mut tape = make_tape(&[instr(HALT, 0), instr(ADD, 0)], 8);
        let steps = Z3::execute(&mut tape, 8192);
        assert_eq!(steps, 1);
    }

    #[test]
    fn test_step_limit() {
        // NOP sled — runs until end of tape or step limit
        let mut tape = vec![NOP; 256]; // 128 words of NOP
        let steps = Z3::execute(&mut tape, 10);
        assert_eq!(steps, 10);
    }

    #[test]
    fn test_runs_to_end_of_tape() {
        // 4 words of NOP => runs exactly 4 steps
        let mut tape = make_tape(
            &[instr(NOP, 0), instr(NOP, 0), instr(NOP, 0), instr(NOP, 0)],
            8,
        );
        let steps = Z3::execute(&mut tape, 8192);
        assert_eq!(steps, 4);
    }

    // ─── No branching: PC always advances sequentially ──────────────────

    #[test]
    fn test_no_branching_purely_sequential() {
        // Every instruction should advance PC by exactly 1 word.
        // Place a sequence: ADD, SUB, MUL, NOP, HALT
        // Should take exactly 5 steps (the 5th is HALT).
        let mut tape = make_tape(
            &[
                instr(ADD, 0),
                instr(SUB, 0),
                instr(MUL, 0),
                instr(NOP, 0),
                instr(HALT, 0),
            ],
            10,
        );
        let steps = Z3::execute(&mut tape, 8192);
        assert_eq!(steps, 5);
    }

    // ─── LOAD / STORE tests ─────────────────────────────────────────────

    #[test]
    fn test_load1() {
        // Word 2 contains value 0x1234. LOAD1 [2] => r1 = 0x1234.
        // STORE1 [3] => mem[3] = r1 = 0x1234.
        let mut tape = make_tape(
            &[
                instr(LOAD1, 2),  // word 0: r1 = mem[2]
                instr(STORE1, 3), // word 1: mem[3] = r1
                (0x34, 0x12),     // word 2: data = 0x1234
                (0x00, 0x00),     // word 3: destination
            ],
            8,
        );
        Z3::execute(&mut tape, 8192);
        assert_eq!(tape[6], 0x34); // word 3 low byte
        assert_eq!(tape[7], 0x12); // word 3 high byte
    }

    #[test]
    fn test_load2() {
        // LOAD2 [4] => r2 = mem[4] = 0x5678. SWAP => r1=0x5678.
        // STORE1 [5] => mem[5] = r1 = 0x5678. HALT.
        let mut tape = make_tape(
            &[
                instr(LOAD2, 4),  // word 0: r2 = mem[4]
                instr(SWAP, 0),   // word 1: r1 <-> r2
                instr(STORE1, 5), // word 2: mem[5] = r1
                instr(HALT, 0),   // word 3
                (0x78, 0x56),     // word 4: data = 0x5678
                (0x00, 0x00),     // word 5: destination
            ],
            12,
        );
        Z3::execute(&mut tape, 8192);
        let result = read_word(&tape, 5, 6);
        assert_eq!(result, 0x5678_i16);
    }

    #[test]
    fn test_store2() {
        // LOAD2 [2] => r2 = mem[2]. STORE2 [3] => mem[3] = r2.
        let mut tape = make_tape(
            &[
                instr(LOAD2, 2),  // word 0
                instr(STORE2, 3), // word 1
                (0xCD, 0xAB),     // word 2: data = 0xABCD
                (0x00, 0x00),     // word 3: destination
            ],
            8,
        );
        Z3::execute(&mut tape, 8192);
        assert_eq!(tape[6], 0xCD);
        assert_eq!(tape[7], 0xAB);
    }

    // ─── Arithmetic tests ───────────────────────────────────────────────

    #[test]
    fn test_add() {
        // r1=5, r2=3 => r1 = 5+3 = 8
        let mut tape = make_tape(
            &[
                instr(LOAD1, 4),  // r1 = mem[4] = 5
                instr(LOAD2, 5),  // r2 = mem[5] = 3
                instr(ADD, 0),    // r1 = r1 + r2 = 8
                instr(STORE1, 6), // mem[6] = r1
                (5, 0),           // word 4: value 5
                (3, 0),           // word 5: value 3
                (0, 0),           // word 6: destination
            ],
            14,
        );
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 6, 7), 8);
    }

    #[test]
    fn test_add_wrapping() {
        // i16::MAX + 1 wraps to i16::MIN
        let mut tape = make_tape(
            &[
                instr(LOAD1, 5),  // r1 = i16::MAX
                instr(LOAD2, 6),  // r2 = 1
                instr(ADD, 0),    // r1 = i16::MAX + 1 = i16::MIN (wrapping)
                instr(STORE1, 7), // mem[7] = r1
                instr(HALT, 0),   // word 4
            ],
            16,
        );
        write_word(&mut tape, 5, 8, i16::MAX);
        write_word(&mut tape, 6, 8, 1);
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 7, 8), i16::MIN);
    }

    #[test]
    fn test_sub() {
        // r1=10, r2=3 => r1 = 10-3 = 7
        let mut tape = make_tape(
            &[
                instr(LOAD1, 4),
                instr(LOAD2, 5),
                instr(SUB, 0),
                instr(STORE1, 6),
                (10, 0),
                (3, 0),
                (0, 0),
            ],
            14,
        );
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 6, 7), 7);
    }

    #[test]
    fn test_mul() {
        // r1=6, r2=7 => r1 = 42
        let mut tape = make_tape(
            &[
                instr(LOAD1, 4),
                instr(LOAD2, 5),
                instr(MUL, 0),
                instr(STORE1, 6),
                (6, 0),
                (7, 0),
                (0, 0),
            ],
            14,
        );
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 6, 7), 42);
    }

    #[test]
    fn test_mul_wrapping() {
        // 1000 * 1000 = 1_000_000, which wraps in i16
        let mut tape = make_tape(
            &[
                instr(LOAD1, 5),
                instr(LOAD2, 6),
                instr(MUL, 0),
                instr(STORE1, 7),
                instr(HALT, 0),
            ],
            16,
        );
        write_word(&mut tape, 5, 8, 1000);
        write_word(&mut tape, 6, 8, 1000);
        Z3::execute(&mut tape, 8192);
        let result = read_word(&tape, 7, 8);
        assert_eq!(result, 1000_i16.wrapping_mul(1000));
    }

    #[test]
    fn test_div() {
        // r1=20, r2=4 => r1 = 5
        let mut tape = make_tape(
            &[
                instr(LOAD1, 4),
                instr(LOAD2, 5),
                instr(DIV, 0),
                instr(STORE1, 6),
                (20, 0),
                (4, 0),
                (0, 0),
            ],
            14,
        );
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 6, 7), 5);
    }

    #[test]
    fn test_div_by_zero() {
        // r1=42, r2=0 => r1 = 0
        let mut tape = make_tape(
            &[
                instr(LOAD1, 4),
                instr(LOAD2, 5),
                instr(DIV, 0),
                instr(STORE1, 6),
                (42, 0),
                (0, 0),
                (0, 0),
            ],
            14,
        );
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 6, 7), 0);
    }

    #[test]
    fn test_div_min_by_neg1() {
        // i16::MIN / -1 would overflow; wrapping_div wraps to i16::MIN
        let mut tape = make_tape(
            &[
                instr(LOAD1, 5),
                instr(LOAD2, 6),
                instr(DIV, 0),
                instr(STORE1, 7),
                instr(HALT, 0),
            ],
            16,
        );
        write_word(&mut tape, 5, 8, i16::MIN);
        write_word(&mut tape, 6, 8, -1);
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 7, 8), i16::MIN);
    }

    // ─── SQRT tests ─────────────────────────────────────────────────────

    #[test]
    fn test_sqrt_4() {
        // sqrt(|4|) = 2
        let mut tape = make_tape(
            &[
                instr(LOAD1, 3),
                instr(SQRT, 0),
                instr(STORE1, 4),
            ],
            10,
        );
        write_word(&mut tape, 3, 5, 4);
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 4, 5), 2);
    }

    #[test]
    fn test_sqrt_9() {
        let mut tape = make_tape(
            &[
                instr(LOAD1, 3),
                instr(SQRT, 0),
                instr(STORE1, 4),
            ],
            10,
        );
        write_word(&mut tape, 3, 5, 9);
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 4, 5), 3);
    }

    #[test]
    fn test_sqrt_0() {
        let mut tape = make_tape(
            &[instr(SQRT, 0), instr(STORE1, 2)],
            6,
        );
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 2, 3), 0);
    }

    #[test]
    fn test_sqrt_1() {
        let mut tape = make_tape(
            &[
                instr(LOAD1, 3),
                instr(SQRT, 0),
                instr(STORE1, 4),
            ],
            10,
        );
        write_word(&mut tape, 3, 5, 1);
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 4, 5), 1);
    }

    #[test]
    fn test_sqrt_100() {
        let mut tape = make_tape(
            &[
                instr(LOAD1, 3),
                instr(SQRT, 0),
                instr(STORE1, 4),
            ],
            10,
        );
        write_word(&mut tape, 3, 5, 100);
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 4, 5), 10);
    }

    #[test]
    fn test_sqrt_negative() {
        // sqrt(|-9|) = sqrt(9) = 3
        let mut tape = make_tape(
            &[
                instr(LOAD1, 3),
                instr(SQRT, 0),
                instr(STORE1, 4),
            ],
            10,
        );
        write_word(&mut tape, 3, 5, -9);
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 4, 5), 3);
    }

    #[test]
    fn test_sqrt_non_perfect() {
        // sqrt(|10|) = 3 (integer truncation)
        let mut tape = make_tape(
            &[
                instr(LOAD1, 3),
                instr(SQRT, 0),
                instr(STORE1, 4),
            ],
            10,
        );
        write_word(&mut tape, 3, 5, 10);
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 4, 5), 3);
    }

    // ─── NEG / ABS tests ────────────────────────────────────────────────

    #[test]
    fn test_neg() {
        // r1=42 => r1 = -42
        let mut tape = make_tape(
            &[
                instr(LOAD1, 3),
                instr(NEG, 0),
                instr(STORE1, 4),
            ],
            10,
        );
        write_word(&mut tape, 3, 5, 42);
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 4, 5), -42);
    }

    #[test]
    fn test_neg_zero() {
        let mut tape = make_tape(
            &[instr(NEG, 0), instr(STORE1, 2)],
            6,
        );
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 2, 3), 0);
    }

    #[test]
    fn test_neg_min() {
        // -i16::MIN wraps to i16::MIN
        let mut tape = make_tape(
            &[
                instr(LOAD1, 3),
                instr(NEG, 0),
                instr(STORE1, 4),
            ],
            10,
        );
        write_word(&mut tape, 3, 5, i16::MIN);
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 4, 5), i16::MIN);
    }

    #[test]
    fn test_abs() {
        // abs(-42) = 42
        let mut tape = make_tape(
            &[
                instr(LOAD1, 3),
                instr(ABS, 0),
                instr(STORE1, 4),
            ],
            10,
        );
        write_word(&mut tape, 3, 5, -42);
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 4, 5), 42);
    }

    #[test]
    fn test_abs_positive() {
        let mut tape = make_tape(
            &[
                instr(LOAD1, 3),
                instr(ABS, 0),
                instr(STORE1, 4),
            ],
            10,
        );
        write_word(&mut tape, 3, 5, 42);
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 4, 5), 42);
    }

    #[test]
    fn test_abs_min_wraps() {
        // abs(i16::MIN) wraps to i16::MIN (wrapping_abs behavior)
        let mut tape = make_tape(
            &[
                instr(LOAD1, 3),
                instr(ABS, 0),
                instr(STORE1, 4),
            ],
            10,
        );
        write_word(&mut tape, 3, 5, i16::MIN);
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 4, 5), i16::MIN);
    }

    // ─── MOD tests ──────────────────────────────────────────────────────

    #[test]
    fn test_mod() {
        // 17 % 5 = 2
        let mut tape = make_tape(
            &[
                instr(LOAD1, 4),
                instr(LOAD2, 5),
                instr(MOD, 0),
                instr(STORE1, 6),
                (17, 0),
                (5, 0),
                (0, 0),
            ],
            14,
        );
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 6, 7), 2);
    }

    #[test]
    fn test_mod_by_zero() {
        // r1=42, r2=0 => r1 = 0
        let mut tape = make_tape(
            &[
                instr(LOAD1, 4),
                instr(LOAD2, 5),
                instr(MOD, 0),
                instr(STORE1, 6),
                (42, 0),
                (0, 0),
                (0, 0),
            ],
            14,
        );
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 6, 7), 0);
    }

    // ─── SWAP tests ─────────────────────────────────────────────────────

    #[test]
    fn test_swap() {
        // r1=5, r2=10. SWAP => r1=10, r2=5. Store both.
        let mut tape = make_tape(
            &[
                instr(LOAD1, 5),  // r1 = 5
                instr(LOAD2, 6),  // r2 = 10
                instr(SWAP, 0),   // r1 <-> r2
                instr(STORE1, 7), // mem[7] = r1 (was r2 = 10)
                instr(STORE2, 8), // mem[8] = r2 (was r1 = 5)
                (5, 0),           // word 5
                (10, 0),          // word 6
                (0, 0),           // word 7
                (0, 0),           // word 8
            ],
            18,
        );
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 7, 9), 10);
        assert_eq!(read_word(&tape, 8, 9), 5);
    }

    // ─── COPY_FWD tests ─────────────────────────────────────────────────

    #[test]
    fn test_copy_fwd() {
        // COPY_FWD [3] copies mem[4] to mem[3].
        let mut tape = make_tape(
            &[
                instr(COPY_FWD, 3), // word 0: mem[3] = mem[4]
                instr(HALT, 0),     // word 1
                (0, 0),             // word 2
                (0, 0),             // word 3: destination
                (0xEF, 0xBE),       // word 4: source = 0xBEEF
            ],
            10,
        );
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 3, 5), 0xBEEFu16 as i16);
    }

    #[test]
    fn test_copy_fwd_wraps() {
        // On a 5-word tape, COPY_FWD [4] copies mem[(4+1)%5 = 0] to mem[4].
        let mut tape = make_tape(
            &[
                instr(COPY_FWD, 4), // word 0: mem[4] = mem[0]
                instr(HALT, 0),     // word 1
                (0, 0),             // word 2
                (0, 0),             // word 3
                (0, 0),             // word 4: destination
            ],
            10,
        );
        Z3::execute(&mut tape, 8192);
        // mem[0] was the COPY_FWD instruction: (0x0E, 0x04) = 0x040E as i16
        assert_eq!(read_word(&tape, 4, 5), 0x040E_i16);
    }

    // ─── Self-modification test ─────────────────────────────────────────

    #[test]
    fn test_self_modification() {
        // The Z3 has no branching, but STORE can modify future instructions.
        // Load a value, then store it over a future instruction word.
        //
        // word 0: LOAD1 [4]   — r1 = mem[4] = 99
        // word 1: STORE1 [3]  — mem[3] = r1 = 99; overwrites the NOP at word 3
        // word 2: LOAD1 [3]   — r1 = mem[3] (now 99, not the original NOP)
        // word 3: NOP          — will be overwritten to 99 before we reach word 2's load
        // word 4: data = 99
        let mut tape = make_tape(
            &[
                instr(LOAD1, 4),  // word 0
                instr(STORE1, 3), // word 1
                instr(LOAD1, 3),  // word 2: loads the modified value
                instr(NOP, 0),    // word 3: will be overwritten
                (99, 0),          // word 4: data
                (0, 0),           // word 5: for storing result
            ],
            12,
        );
        // After word 1 executes, mem[3] = 99 (as i16, LE bytes: 0x63, 0x00).
        // Word 2 then loads mem[3] = 99. We need to store r1 somewhere to verify.
        // But word 3 is now 99 (opcode = 99 & 0xF = 3 = STORE1, operand = 0).
        // When pc reaches word 3, it executes STORE1 [0] => mem[0] = r1 = 99.
        Z3::execute(&mut tape, 8192);
        // mem[0] should now be 99 (the self-modified STORE1 wrote r1=99 there)
        assert_eq!(read_word(&tape, 0, 6), 99);
    }

    // ─── is_instruction tests ───────────────────────────────────────────

    #[test]
    fn test_is_instruction() {
        // Opcodes 0x0-0xE are instructions
        for opcode in 0x0..=0xEu8 {
            assert!(Z3::is_instruction(opcode), "opcode {opcode:#X} should be instruction");
        }
        // 0xF (NOP) is not counted as a meaningful instruction
        assert!(!Z3::is_instruction(0x0F));

        // High bits don't matter — only low 4 bits
        assert!(Z3::is_instruction(0xF0)); // low 4 bits = 0 = HALT
        assert!(Z3::is_instruction(0x35)); // low 4 bits = 5 = ADD
        assert!(!Z3::is_instruction(0xFF)); // low 4 bits = F = NOP
    }

    // ─── Disassemble tests ──────────────────────────────────────────────

    #[test]
    fn test_disassemble_basic() {
        let tape = vec![
            HALT, 0x00,   // HALT
            LOAD1, 0x05,  // LOAD1 [5]
            ADD, 0x00,    // ADD
            SQRT, 0x00,   // SQRT
            NOP, 0x00,    // NOP
        ];
        let disasm = Z3::disassemble(&tape);
        assert!(disasm.contains("HALT"));
        assert!(disasm.contains("LOAD1 [5]"));
        assert!(disasm.contains("ADD"));
        assert!(disasm.contains("SQRT"));
        assert!(disasm.contains("NOP"));
    }

    #[test]
    fn test_disassemble_empty() {
        let tape: Vec<u8> = vec![];
        let disasm = Z3::disassemble(&tape);
        assert!(disasm.is_empty());
    }

    #[test]
    fn test_disassemble_odd_length() {
        // Odd tape: last byte ignored (not a complete word)
        let tape = vec![ADD, 0x00, 0xFF];
        let disasm = Z3::disassemble(&tape);
        assert!(disasm.contains("ADD"));
        // Should have exactly 1 line (only 1 complete word)
        assert_eq!(disasm.lines().count(), 1);
    }

    // ─── Combined arithmetic sequence ───────────────────────────────────

    #[test]
    fn test_arithmetic_sequence() {
        // Compute: (5 + 3) * 2 - 1 = 15
        // r1=5, r2=3, ADD => r1=8
        // r2=2, MUL => r1=16
        // r2=1, SUB => r1=15
        let mut tape = make_tape(
            &[
                instr(LOAD1, 7),  // r1 = 5
                instr(LOAD2, 8),  // r2 = 3
                instr(ADD, 0),    // r1 = 8
                instr(LOAD2, 9),  // r2 = 2
                instr(MUL, 0),    // r1 = 16
                instr(LOAD2, 10), // r2 = 1
                instr(SUB, 0),    // r1 = 15
                (5, 0),           // word 7
                (3, 0),           // word 8
                (2, 0),           // word 9
                (1, 0),           // word 10
            ],
            22,
        );
        Z3::execute(&mut tape, 8192);
        // r1 = 15, but we didn't store it. Let's add a store.
        let mut tape = make_tape(
            &[
                instr(LOAD1, 8),  // r1 = 5
                instr(LOAD2, 9),  // r2 = 3
                instr(ADD, 0),    // r1 = 8
                instr(LOAD2, 10), // r2 = 2
                instr(MUL, 0),    // r1 = 16
                instr(LOAD2, 11), // r2 = 1
                instr(SUB, 0),    // r1 = 15
                instr(STORE1, 12),// mem[12] = 15
                (5, 0),           // word 8
                (3, 0),           // word 9
                (2, 0),           // word 10
                (1, 0),           // word 11
                (0, 0),           // word 12: result
            ],
            26,
        );
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 12, 13), 15);
    }

    // ─── Address wrapping tests ─────────────────────────────────────────

    #[test]
    fn test_load_address_wraps() {
        // On a 4-word tape, LOAD1 [5] wraps to LOAD1 [5 % 4 = 1]
        let mut tape = make_tape(
            &[
                instr(LOAD1, 5),  // wraps to word 1
                instr(STORE1, 3), // store r1 to word 3
                (0, 0),           // word 2
                (0, 0),           // word 3: destination
            ],
            8,
        );
        // word 1 = STORE1 instruction bytes
        Z3::execute(&mut tape, 8192);
        // r1 should contain the value at word 1 (STORE1 [3] = bytes 0x03, 0x03)
        let word1_val = read_word(&tape, 1, 4);
        assert_eq!(read_word(&tape, 3, 4), word1_val);
    }

    // ─── Register initialization tests ──────────────────────────────────

    #[test]
    fn test_registers_start_at_zero() {
        // ADD with r1=0, r2=0 => r1 stays 0. STORE1 should write 0.
        let mut tape = make_tape(
            &[
                instr(ADD, 0),
                instr(STORE1, 2),
                (0xFF, 0xFF), // word 2: will be overwritten with 0
            ],
            6,
        );
        Z3::execute(&mut tape, 8192);
        assert_eq!(read_word(&tape, 2, 3), 0);
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
            let steps = Z3::execute(&mut tape, 8192);
            prop_assert!(steps <= 8192);
        }

        #[test]
        fn random_programs_respect_step_limit(
            tape_data in prop::collection::vec(any::<u8>(), 1..256),
            limit in 1usize..1000
        ) {
            let mut tape = tape_data;
            let steps = Z3::execute(&mut tape, limit);
            prop_assert!(steps <= limit);
        }

        #[test]
        fn output_tape_same_length(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let original_len = tape_data.len();
            let mut tape = tape_data;
            Z3::execute(&mut tape, 8192);
            prop_assert_eq!(tape.len(), original_len);
        }

        #[test]
        fn pc_always_advances_sequentially(tape_data in prop::collection::vec(any::<u8>(), 2..256)) {
            // The Z3 has no branching, so execution should always be at most
            // num_words steps (each step advances PC by 1, and we stop at end of tape).
            let num_words = tape_data.len() / 2;
            let mut tape = tape_data;
            let steps = Z3::execute(&mut tape, 100_000);
            // Steps can be at most num_words (run off end) or fewer (HALT encountered)
            prop_assert!(steps <= num_words);
        }

        #[test]
        fn sqrt_is_correct(val in any::<i16>()) {
            let abs_val = (val as i32).unsigned_abs();
            let result = isqrt(val.wrapping_abs());
            // result^2 <= |val| < (result+1)^2
            let r = result as u32;
            prop_assert!(r * r <= abs_val);
            prop_assert!((r + 1) * (r + 1) > abs_val);
        }
    }
}
