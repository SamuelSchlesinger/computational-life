use crate::substrate::Substrate;

/// The Bits (Bit-Serial Machine) instruction set — operates on individual bits.
///
/// Unlike all other substrates which operate at byte granularity, Bits addresses
/// individual bits within the tape. This creates a fundamentally different
/// computational granularity.
///
/// State:
/// - `pc`: instruction pointer (byte-addressed), starts at 0
/// - `bp`: bit read pointer, starts at 0
/// - `wp`: bit write pointer, starts at `tape.len() * 8 / 2` (midpoint in bits)
/// - `carry`: 1-bit register, starts at 0
///
/// Bit addressing: bit `n` is `(tape[n / 8] >> (n % 8)) & 1`.
/// All bit pointers wrap modulo `tape.len() * 8`.
pub struct Bits;

#[inline(always)]
fn total_bits(tape: &[u8]) -> usize {
    tape.len() * 8
}

#[inline(always)]
fn read_bit(tape: &[u8], bit_pos: usize) -> u8 {
    let tb = total_bits(tape);
    if tb == 0 {
        return 0;
    }
    let pos = bit_pos % tb;
    (tape[pos / 8] >> (pos % 8)) & 1
}

#[inline(always)]
fn write_bit(tape: &mut [u8], bit_pos: usize, val: u8) {
    let tb = total_bits(tape);
    if tb == 0 {
        return;
    }
    let pos = bit_pos % tb;
    let byte_idx = pos / 8;
    let bit_idx = pos % 8;
    if val & 1 == 1 {
        tape[byte_idx] |= 1 << bit_idx;
    } else {
        tape[byte_idx] &= !(1 << bit_idx);
    }
}

struct BitsBattleState {
    pc: usize,
    bp: usize,
    wp: usize,
    carry: u8,
}

/// Execute one Bits instruction for a battle context. Returns true if still running.
fn bits_battle_step(state: &mut BitsBattleState, tape: &mut [u8]) -> bool {
    let len = tape.len();
    if state.pc >= len {
        return false;
    }
    let tb = total_bits(tape);
    let instr = tape[state.pc];

    match instr >> 4 {
        0x0 => {
            // COPY_BIT
            let bit = read_bit(tape, state.bp);
            write_bit(tape, state.wp, bit);
            state.bp = (state.bp + 1) % tb;
            state.wp = (state.wp + 1) % tb;
        }
        0x1 => {
            // SET_BIT
            write_bit(tape, state.wp, 1);
            state.wp = (state.wp + 1) % tb;
        }
        0x2 => {
            // CLR_BIT
            write_bit(tape, state.wp, 0);
            state.wp = (state.wp + 1) % tb;
        }
        0x3 => {
            // SKIP_BIT
            state.bp = (state.bp + 1) % tb;
        }
        0x4 => {
            // READ_CARRY
            state.carry = read_bit(tape, state.bp);
            state.bp = (state.bp + 1) % tb;
        }
        0x5 => {
            // WRITE_CARRY
            write_bit(tape, state.wp, state.carry);
            state.wp = (state.wp + 1) % tb;
        }
        0x6 => {
            // FLIP_CARRY
            state.carry ^= 1;
        }
        0x7 => {
            // AND_CARRY
            state.carry &= read_bit(tape, state.bp);
            state.bp = (state.bp + 1) % tb;
        }
        0x8 => {
            // OR_CARRY
            state.carry |= read_bit(tape, state.bp);
            state.bp = (state.bp + 1) % tb;
        }
        0x9 => {
            // XOR_CARRY
            state.carry ^= read_bit(tape, state.bp);
            state.bp = (state.bp + 1) % tb;
        }
        0xA => {
            // JZ_CARRY
            if state.pc + 1 >= len {
                return false;
            }
            if state.carry == 0 {
                let offset = tape[state.pc + 1] as i8;
                let new_pc = state.pc as isize + 2 + offset as isize;
                if new_pc < 0 {
                    return false;
                }
                state.pc = new_pc as usize;
                return true;
            }
            state.pc += 2;
            return true;
        }
        0xB => {
            // JNZ_CARRY
            if state.pc + 1 >= len {
                return false;
            }
            if state.carry != 0 {
                let offset = tape[state.pc + 1] as i8;
                let new_pc = state.pc as isize + 2 + offset as isize;
                if new_pc < 0 {
                    return false;
                }
                state.pc = new_pc as usize;
                return true;
            }
            state.pc += 2;
            return true;
        }
        0xC => {
            // BP_RESET
            state.bp = 0;
        }
        0xD => {
            // WP_RESET
            state.wp = total_bits(tape) / 2;
        }
        0xE => {
            // HALT
            return false;
        }
        // 0xF: NOP
        _ => {}
    }

    state.pc += 1;
    true
}

impl Substrate for Bits {
    fn execute(tape: &mut [u8], step_limit: usize) -> usize {
        let len = tape.len();
        if len == 0 {
            return 0;
        }

        let tb = total_bits(tape);
        let mut pc: usize = 0;
        let mut bp: usize = 0;
        let mut wp: usize = tb / 2;
        let mut carry: u8 = 0;
        let mut steps: usize = 0;

        while pc < len && steps < step_limit {
            steps += 1;
            let instr = tape[pc];

            match instr >> 4 {
                0x0 => {
                    // COPY_BIT: write_bit(wp, read_bit(bp)); bp++; wp++
                    let bit = read_bit(tape, bp);
                    write_bit(tape, wp, bit);
                    bp = (bp + 1) % tb;
                    wp = (wp + 1) % tb;
                }
                0x1 => {
                    // SET_BIT: write 1 to wp; wp++
                    write_bit(tape, wp, 1);
                    wp = (wp + 1) % tb;
                }
                0x2 => {
                    // CLR_BIT: write 0 to wp; wp++
                    write_bit(tape, wp, 0);
                    wp = (wp + 1) % tb;
                }
                0x3 => {
                    // SKIP_BIT: bp++
                    bp = (bp + 1) % tb;
                }
                0x4 => {
                    // READ_CARRY: carry = read_bit(bp); bp++
                    carry = read_bit(tape, bp);
                    bp = (bp + 1) % tb;
                }
                0x5 => {
                    // WRITE_CARRY: write carry to wp; wp++
                    write_bit(tape, wp, carry);
                    wp = (wp + 1) % tb;
                }
                0x6 => {
                    // FLIP_CARRY: carry ^= 1
                    carry ^= 1;
                }
                0x7 => {
                    // AND_CARRY: carry &= read_bit(bp); bp++
                    carry &= read_bit(tape, bp);
                    bp = (bp + 1) % tb;
                }
                0x8 => {
                    // OR_CARRY: carry |= read_bit(bp); bp++
                    carry |= read_bit(tape, bp);
                    bp = (bp + 1) % tb;
                }
                0x9 => {
                    // XOR_CARRY: carry ^= read_bit(bp); bp++
                    carry ^= read_bit(tape, bp);
                    bp = (bp + 1) % tb;
                }
                0xA => {
                    // JZ_CARRY: 2-byte instruction. If carry==0, relative jump.
                    if pc + 1 >= len {
                        break;
                    }
                    if carry == 0 {
                        let offset = tape[pc + 1] as i8;
                        let new_pc = pc as isize + 2 + offset as isize;
                        if new_pc < 0 {
                            break;
                        }
                        pc = new_pc as usize;
                        continue;
                    }
                    pc += 2;
                    continue;
                }
                0xB => {
                    // JNZ_CARRY: 2-byte instruction. If carry!=0, relative jump.
                    if pc + 1 >= len {
                        break;
                    }
                    if carry != 0 {
                        let offset = tape[pc + 1] as i8;
                        let new_pc = pc as isize + 2 + offset as isize;
                        if new_pc < 0 {
                            break;
                        }
                        pc = new_pc as usize;
                        continue;
                    }
                    pc += 2;
                    continue;
                }
                0xC => {
                    // BP_RESET: bp = 0
                    bp = 0;
                }
                0xD => {
                    // WP_RESET: wp = total_bits / 2
                    wp = tb / 2;
                }
                0xE => {
                    // HALT
                    break;
                }
                // 0xF: NOP
                _ => {}
            }

            pc += 1;
        }

        steps
    }

    fn execute_battle(tape: &mut [u8], ps: usize, step_limit: usize) -> usize {
        let len = tape.len();
        if len == 0 {
            return 0;
        }
        let mut a = BitsBattleState {
            pc: 0,
            bp: 0,
            wp: ps * 8,
            carry: 0,
        };
        let mut b = BitsBattleState {
            pc: ps,
            bp: ps * 8,
            wp: 0,
            carry: 0,
        };
        let mut steps = 0;
        let mut halted_a = false;
        let mut halted_b = false;
        while steps < step_limit && (!halted_a || !halted_b) {
            if !halted_a {
                halted_a = !bits_battle_step(&mut a, tape);
                steps += 1;
                if steps >= step_limit {
                    break;
                }
            }
            if !halted_b {
                halted_b = !bits_battle_step(&mut b, tape);
                steps += 1;
            }
        }
        steps
    }

    fn is_instruction(byte: u8) -> bool {
        (byte >> 4) <= 0xE
    }

    fn disassemble(tape: &[u8]) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        let mut pc = 0;
        while pc < tape.len() {
            let b = tape[pc];
            let desc = match b >> 4 {
                0x0 => {
                    pc += 1;
                    "COPY_BIT".to_string()
                }
                0x1 => {
                    pc += 1;
                    "SET_BIT".to_string()
                }
                0x2 => {
                    pc += 1;
                    "CLR_BIT".to_string()
                }
                0x3 => {
                    pc += 1;
                    "SKIP_BIT".to_string()
                }
                0x4 => {
                    pc += 1;
                    "READ_CARRY".to_string()
                }
                0x5 => {
                    pc += 1;
                    "WRITE_CARRY".to_string()
                }
                0x6 => {
                    pc += 1;
                    "FLIP_CARRY".to_string()
                }
                0x7 => {
                    pc += 1;
                    "AND_CARRY".to_string()
                }
                0x8 => {
                    pc += 1;
                    "OR_CARRY".to_string()
                }
                0x9 => {
                    pc += 1;
                    "XOR_CARRY".to_string()
                }
                0xA => {
                    if pc + 1 < tape.len() {
                        let offset = tape[pc + 1] as i8;
                        let target = pc as isize + 2 + offset as isize;
                        let s = format!("JZ_CARRY {offset:+} (-> {target})");
                        pc += 2;
                        s
                    } else {
                        pc += 1;
                        "JZ_CARRY ???".to_string()
                    }
                }
                0xB => {
                    if pc + 1 < tape.len() {
                        let offset = tape[pc + 1] as i8;
                        let target = pc as isize + 2 + offset as isize;
                        let s = format!("JNZ_CARRY {offset:+} (-> {target})");
                        pc += 2;
                        s
                    } else {
                        pc += 1;
                        "JNZ_CARRY ???".to_string()
                    }
                }
                0xC => {
                    pc += 1;
                    "BP_RESET".to_string()
                }
                0xD => {
                    pc += 1;
                    "WP_RESET".to_string()
                }
                0xE => {
                    pc += 1;
                    "HALT".to_string()
                }
                _ => {
                    pc += 1;
                    "NOP".to_string()
                }
            };
            let display_pc = if matches!(b >> 4, 0xA | 0xB) && pc >= 2 {
                pc - 2
            } else {
                pc - 1
            };
            let _ = writeln!(out, "{display_pc:04X}: {b:02X}  {desc}");
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tape(program: &[u8], size: usize) -> Vec<u8> {
        let mut tape = vec![0u8; size];
        for (i, &b) in program.iter().enumerate() {
            if i < size {
                tape[i] = b;
            }
        }
        tape
    }

    #[test]
    fn test_halt() {
        let mut tape = make_tape(&[0xE0], 128);
        let steps = Bits::execute(&mut tape, 8192);
        assert_eq!(steps, 1);
    }

    #[test]
    fn test_copy_bit() {
        // Set tape[0] = 0x05 (bits: 10100000). COPY_BIT copies bit 0 (=1) to bit 512 (= byte 64, bit 0).
        let mut tape = make_tape(&[0x05], 128);
        // tape[0] = 0x05, but COPY_BIT is opcode 0x0_, so tape[0] must be 0x0_.
        // 0x05 >> 4 = 0x0, so it IS a COPY_BIT. It copies bit bp=0 to bit wp=512.
        // bit 0 of tape[0] = 1 (0x05 & 1 = 1). bit 512 = byte 64, bit 0.
        // After: tape[64] should have bit 0 set.
        Bits::execute(&mut tape, 1);
        assert_eq!(tape[64] & 1, 1);
    }

    #[test]
    fn test_set_bit() {
        // SET_BIT: write 1 to wp (bit 512 = byte 64 bit 0).
        let mut tape = make_tape(&[0x10, 0xE0], 128);
        Bits::execute(&mut tape, 8192);
        assert_eq!(tape[64] & 1, 1);
    }

    #[test]
    fn test_clr_bit() {
        // Set tape[64] = 0xFF. CLR_BIT clears bit 512 (byte 64 bit 0).
        let mut tape = make_tape(&[0x20, 0xE0], 128);
        tape[64] = 0xFF;
        Bits::execute(&mut tape, 8192);
        assert_eq!(tape[64], 0xFE); // bit 0 cleared
    }

    #[test]
    fn test_skip_bit() {
        // SKIP_BIT advances bp. Then READ_CARRY reads bit 1 (not bit 0).
        // tape[0] = SKIP_BIT = 0x30. Bit 0 = 0, bit 1 = 0, bit 4 = 1, bit 5 = 1.
        // After SKIP_BIT: bp=1. READ_CARRY at pc=1: carry = bit 1 of tape[0] = 0.
        let mut tape = make_tape(&[0x30, 0x40, 0xE0], 128);
        // Bit 1 of 0x30 = 0. So carry = 0. Nothing observable without writing.
        Bits::execute(&mut tape, 8192);
        // Just verify no panic.
    }

    #[test]
    fn test_read_carry() {
        // Set tape[0] bit pattern. READ_CARRY reads bit 0 into carry.
        // tape[0] = 0x41 (READ_CARRY opcode). Bit 0 = 1. carry becomes 1.
        // WRITE_CARRY writes carry to wp (bit 512 = byte 64 bit 0).
        let mut tape = make_tape(&[0x41, 0x50, 0xE0], 128);
        Bits::execute(&mut tape, 8192);
        assert_eq!(tape[64] & 1, 1);
    }

    #[test]
    fn test_write_carry() {
        // FLIP_CARRY (carry=1), WRITE_CARRY (bit 512 = 1).
        let mut tape = make_tape(&[0x60, 0x50, 0xE0], 128);
        Bits::execute(&mut tape, 8192);
        assert_eq!(tape[64] & 1, 1);
    }

    #[test]
    fn test_flip_carry() {
        // FLIP_CARRY twice: carry goes 0->1->0. WRITE_CARRY should write 0.
        let mut tape = make_tape(&[0x60, 0x60, 0x50, 0xE0], 128);
        tape[64] = 0xFF;
        Bits::execute(&mut tape, 8192);
        assert_eq!(tape[64] & 1, 0); // bit 0 cleared
    }

    #[test]
    fn test_and_carry() {
        // FLIP_CARRY (carry=1). AND_CARRY with bit 0 of tape.
        // tape[0] = 0x60 = FLIP_CARRY. Bit 0 of 0x60 = 0. carry = 1 & 0 = 0.
        // Next: tape[1] = AND_CARRY. bp was 0, AND reads bit bp=0, then bp=1.
        // Wait, FLIP_CARRY doesn't advance bp. So bp=0 when AND executes.
        // tape[0] = 0x60: bit 0 = 0. carry = 1 & 0 = 0.
        // WRITE_CARRY writes 0 to wp.
        let mut tape = make_tape(&[0x60, 0x70, 0x50, 0xE0], 128);
        tape[64] = 0xFF;
        Bits::execute(&mut tape, 8192);
        assert_eq!(tape[64] & 1, 0);
    }

    #[test]
    fn test_or_carry() {
        // carry=0. OR_CARRY with bit 0.
        // tape[0] = 0x81 (OR_CARRY). Bit 0 = 1. carry = 0 | 1 = 1.
        // WRITE_CARRY writes 1 to bit 512.
        let mut tape = make_tape(&[0x81, 0x50, 0xE0], 128);
        Bits::execute(&mut tape, 8192);
        assert_eq!(tape[64] & 1, 1);
    }

    #[test]
    fn test_xor_carry() {
        // FLIP_CARRY (carry=1). XOR_CARRY with bit 0.
        // tape[0] = 0x60. bit 0 = 0. carry = 1 ^ 0 = 1.
        // tape[1] = XOR_CARRY = 0x90. bp=0 (FLIP doesn't advance bp).
        // bit 0 of tape[0] = 0. carry = 1 ^ 0 = 1.
        // WRITE_CARRY writes 1.
        let mut tape = make_tape(&[0x60, 0x90, 0x50, 0xE0], 128);
        Bits::execute(&mut tape, 8192);
        assert_eq!(tape[64] & 1, 1);
    }

    #[test]
    fn test_jz_carry_taken() {
        // carry=0. JZ_CARRY with offset +1: skip 1 byte, land on HALT.
        let mut tape = make_tape(&[0xA0, 0x01, 0xF0, 0xE0], 128);
        let steps = Bits::execute(&mut tape, 8192);
        // JZ at 0: carry==0, jump to 0+2+1=3. HALT at 3.
        assert_eq!(steps, 2); // JZ + HALT
    }

    #[test]
    fn test_jz_carry_not_taken() {
        // FLIP_CARRY (carry=1). JZ_CARRY: carry!=0, fall through.
        let mut tape = make_tape(&[0x60, 0xA0, 0x02, 0xE0], 128);
        let steps = Bits::execute(&mut tape, 8192);
        // FLIP at 0. JZ at 1: carry=1, not taken, pc=3. HALT at 3.
        assert_eq!(steps, 3);
    }

    #[test]
    fn test_jnz_carry_taken() {
        // FLIP_CARRY (carry=1). JNZ_CARRY offset +1: skip NOP, land on HALT.
        let mut tape = make_tape(&[0x60, 0xB0, 0x01, 0xF0, 0xE0], 128);
        let steps = Bits::execute(&mut tape, 8192);
        // FLIP at 0. JNZ at 1: carry=1, jump to 1+2+1=4. HALT at 4.
        assert_eq!(steps, 3); // FLIP + JNZ + HALT
    }

    #[test]
    fn test_jnz_carry_not_taken() {
        // carry=0. JNZ_CARRY: not taken, fall through.
        let mut tape = make_tape(&[0xB0, 0x01, 0xE0], 128);
        let steps = Bits::execute(&mut tape, 8192);
        // JNZ at 0: carry=0, not taken, pc=2. HALT at 2.
        assert_eq!(steps, 2);
    }

    #[test]
    fn test_jnz_backward() {
        // Infinite loop: FLIP_CARRY, JNZ_CARRY back to start.
        // FLIP at 0, JNZ at 1 with offset -3: pc = 1+2+(-3) = 0.
        let mut tape = make_tape(&[0x60, 0xB0, 0xFD], 128);
        let steps = Bits::execute(&mut tape, 100);
        assert_eq!(steps, 100);
    }

    #[test]
    fn test_bp_reset() {
        // SKIP_BIT (bp=1), SKIP_BIT (bp=2), BP_RESET (bp=0).
        // READ_CARRY at bp=0: reads bit 0 of tape[0].
        // tape[0] = 0x30 = SKIP_BIT. Bit 0 = 0.
        // WRITE_CARRY writes 0.
        let mut tape = make_tape(&[0x30, 0x30, 0xC0, 0x40, 0x50, 0xE0], 128);
        tape[64] = 0xFF;
        Bits::execute(&mut tape, 8192);
        assert_eq!(tape[64] & 1, 0); // carry was 0, cleared bit 0 of tape[64]
    }

    #[test]
    fn test_wp_reset() {
        // SET_BIT (wp=513), WP_RESET (wp=512), CLR_BIT (wp=512, clears bit 512).
        let mut tape = make_tape(&[0x10, 0xD0, 0x20, 0xE0], 128);
        Bits::execute(&mut tape, 8192);
        // SET_BIT wrote 1 to bit 512 (byte 64 bit 0). WP_RESET moved wp back to 512.
        // CLR_BIT cleared bit 512 again. So byte 64 bit 0 = 0.
        assert_eq!(tape[64] & 1, 0);
    }

    #[test]
    fn test_nop() {
        let mut tape = make_tape(&[0xF0, 0xF0, 0xE0], 128);
        let steps = Bits::execute(&mut tape, 8192);
        assert_eq!(steps, 3);
    }

    #[test]
    fn test_empty_tape() {
        let mut tape: Vec<u8> = vec![];
        let steps = Bits::execute(&mut tape, 8192);
        assert_eq!(steps, 0);
    }

    #[test]
    fn test_step_limit() {
        // FLIP + JNZ back = infinite loop.
        let mut tape = make_tape(&[0x60, 0xB0, 0xFD], 128);
        let steps = Bits::execute(&mut tape, 50);
        assert_eq!(steps, 50);
    }

    // --- Replicator tests ---

    #[test]
    fn test_4_byte_replicator() {
        // COPY_BIT (0x00), FLIP_CARRY (0x60), JNZ_CARRY (0xB0), offset -4 (0xFC).
        // Copies one bit per iteration. After 512 iterations, all 64 bytes copied.
        let replicator: [u8; 4] = [0x00, 0x60, 0xB0, 0xFC];

        let mut tape = vec![0u8; 128];
        tape[..4].copy_from_slice(&replicator);

        Bits::execute(&mut tape, 8192);

        // First 4 bytes of second half should match the replicator.
        assert_eq!(
            &tape[64..68],
            &replicator,
            "replicator bytes should be copied"
        );
        // Rest should be zeros.
        assert_eq!(&tape[68..128], &vec![0u8; 60], "padding should be zeros");
    }

    #[test]
    fn test_replicator_is_functional_fixed_point() {
        let replicator: [u8; 4] = [0x00, 0x60, 0xB0, 0xFC];

        // Run 1
        let mut tape1 = vec![0u8; 128];
        tape1[..4].copy_from_slice(&replicator);
        Bits::execute(&mut tape1, 8192);
        let copy1 = tape1[64..128].to_vec();

        // Run 2
        let mut tape2 = vec![0u8; 128];
        tape2[..64].copy_from_slice(&copy1);
        Bits::execute(&mut tape2, 8192);
        let copy2 = tape2[64..128].to_vec();

        assert_eq!(copy1, copy2, "Bits replicator should be a fixed point");
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
            let steps = Bits::execute(&mut tape, 8192);
            prop_assert!(steps <= 8192);
        }

        #[test]
        fn random_programs_respect_step_limit(
            tape_data in prop::collection::vec(any::<u8>(), 1..256),
            limit in 1usize..1000
        ) {
            let mut tape = tape_data;
            let steps = Bits::execute(&mut tape, limit);
            prop_assert!(steps <= limit);
        }

        #[test]
        fn output_tape_same_length(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let original_len = tape_data.len();
            let mut tape = tape_data;
            Bits::execute(&mut tape, 8192);
            prop_assert_eq!(tape.len(), original_len);
        }
    }
}
