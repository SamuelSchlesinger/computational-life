use crate::substrate::Substrate;

/// The Skim (Skip-Chain Machine) instruction set — a novel substrate where
/// every byte is simultaneously an opcode AND a jump distance.
///
/// There is no sequential PC increment. The low 4 bits of each instruction
/// determine how far to skip for the next instruction: `pc = (pc + low_nibble + 1) % len`.
/// The high 4 bits determine the operation.
///
/// This means data IS control flow — every byte in the tape, including data
/// regions, redirects execution when reached.
///
/// State:
/// - `pc`: instruction pointer, starts at 0
/// - `acc`: accumulator register (u8), starts at 0
/// - `wp`: write pointer (u8), starts at `tape.len() / 2`
///
/// All pointer arithmetic wraps modulo tape length.
pub struct Skim;

struct SkimBattleState {
    pc: usize,
    acc: u8,
    wp: u8,
}

/// Execute one Skim instruction for a battle context. Returns true if still running.
fn skim_battle_step(state: &mut SkimBattleState, tape: &mut [u8]) -> bool {
    let len = tape.len();
    let instr = tape[state.pc];
    let skip = (instr & 0x0F) as usize + 1;

    match instr >> 4 {
        0x0 => {
            // LOAD: acc = tape[wp]
            state.acc = tape[state.wp as usize % len];
        }
        0x1 => {
            // STORE: tape[wp] = acc
            tape[state.wp as usize % len] = state.acc;
        }
        0x2 => {
            // COPY_FWD: tape[wp] = tape[pc]; wp++
            tape[state.wp as usize % len] = tape[state.pc];
            state.wp = state.wp.wrapping_add(1);
        }
        0x3 => {
            // INC: acc++
            state.acc = state.acc.wrapping_add(1);
        }
        0x4 => {
            // DEC: acc--
            state.acc = state.acc.wrapping_sub(1);
        }
        0x5 => {
            // XOR: acc ^= tape[wp]
            state.acc ^= tape[state.wp as usize % len];
        }
        0x6 => {
            // WP_INC: wp++
            state.wp = state.wp.wrapping_add(1);
        }
        0x7 => {
            // WP_DEC: wp--
            state.wp = state.wp.wrapping_sub(1);
        }
        0x8 => {
            // SET_WP: wp = acc
            state.wp = state.acc;
        }
        0x9 => {
            // GET_WP: acc = wp
            state.acc = state.wp;
        }
        0xA => {
            // SKZ: if acc == 0, use normal skip; else skip 1
            if state.acc != 0 {
                state.pc = (state.pc + 1) % len;
                return true;
            }
        }
        0xB => {
            // SKNZ: if acc != 0, use normal skip; else skip 1
            if state.acc == 0 {
                state.pc = (state.pc + 1) % len;
                return true;
            }
        }
        0xC => {
            // HALT
            return false;
        }
        // 0xD, 0xE, 0xF => NOP
        _ => {}
    }

    state.pc = (state.pc + skip) % len;
    true
}

impl Substrate for Skim {
    fn execute(tape: &mut [u8], step_limit: usize) -> usize {
        let len = tape.len();
        if len == 0 {
            return 0;
        }

        let mut pc: usize = 0;
        let mut acc: u8 = 0;
        let mut wp: u8 = (len / 2) as u8;
        let mut steps: usize = 0;

        while steps < step_limit {
            steps += 1;
            let instr = tape[pc];
            let skip = (instr & 0x0F) as usize + 1;

            match instr >> 4 {
                0x0 => {
                    // LOAD: acc = tape[wp]
                    acc = tape[wp as usize % len];
                }
                0x1 => {
                    // STORE: tape[wp] = acc
                    tape[wp as usize % len] = acc;
                }
                0x2 => {
                    // COPY_FWD: tape[wp] = tape[pc]; wp++
                    tape[wp as usize % len] = tape[pc];
                    wp = wp.wrapping_add(1);
                }
                0x3 => {
                    // INC: acc++
                    acc = acc.wrapping_add(1);
                }
                0x4 => {
                    // DEC: acc--
                    acc = acc.wrapping_sub(1);
                }
                0x5 => {
                    // XOR: acc ^= tape[wp]
                    acc ^= tape[wp as usize % len];
                }
                0x6 => {
                    // WP_INC: wp++
                    wp = wp.wrapping_add(1);
                }
                0x7 => {
                    // WP_DEC: wp--
                    wp = wp.wrapping_sub(1);
                }
                0x8 => {
                    // SET_WP: wp = acc
                    wp = acc;
                }
                0x9 => {
                    // GET_WP: acc = wp
                    acc = wp;
                }
                0xA => {
                    // SKZ: if acc == 0, use normal skip; else skip 1
                    if acc != 0 {
                        pc = (pc + 1) % len;
                        continue;
                    }
                }
                0xB => {
                    // SKNZ: if acc != 0, use normal skip; else skip 1
                    if acc == 0 {
                        pc = (pc + 1) % len;
                        continue;
                    }
                }
                0xC => {
                    // HALT
                    break;
                }
                // 0xD, 0xE, 0xF => NOP
                _ => {}
            }

            pc = (pc + skip) % len;
        }

        steps
    }

    fn execute_battle(tape: &mut [u8], ps: usize, step_limit: usize) -> usize {
        let len = tape.len();
        if len == 0 {
            return 0;
        }
        let mut a = SkimBattleState {
            pc: 0,
            acc: 0,
            wp: ps as u8,
        };
        let mut b = SkimBattleState {
            pc: ps,
            acc: 0,
            wp: 0,
        };
        let mut steps = 0;
        let mut halted_a = false;
        let mut halted_b = false;
        while steps < step_limit && (!halted_a || !halted_b) {
            if !halted_a {
                halted_a = !skim_battle_step(&mut a, tape);
                steps += 1;
                if steps >= step_limit {
                    break;
                }
            }
            if !halted_b {
                halted_b = !skim_battle_step(&mut b, tape);
                steps += 1;
            }
        }
        steps
    }

    fn is_instruction(byte: u8) -> bool {
        // 0xD_, 0xE_, 0xF_ are NOPs; everything else is meaningful.
        (byte >> 4) <= 0xC
    }

    fn disassemble(tape: &[u8]) -> String {
        use std::fmt::Write;
        let len = tape.len();
        let mut out = String::new();
        for (addr, &b) in tape.iter().enumerate() {
            let skip = (b & 0x0F) as usize + 1;
            let target = (addr + skip) % len;
            let op = match b >> 4 {
                0x0 => "LOAD",
                0x1 => "STORE",
                0x2 => "COPY_FWD",
                0x3 => "INC",
                0x4 => "DEC",
                0x5 => "XOR",
                0x6 => "WP_INC",
                0x7 => "WP_DEC",
                0x8 => "SET_WP",
                0x9 => "GET_WP",
                0xA => "SKZ",
                0xB => "SKNZ",
                0xC => "HALT",
                _ => "NOP",
            };
            let _ = writeln!(
                out,
                "{addr:04X}: {b:02X}  {op:<10} skip {skip} -> {target:04X}"
            );
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

    // --- Basic instruction tests ---

    #[test]
    fn test_halt() {
        // 0xC0 = HALT with skip 1. Should stop immediately.
        let mut tape = make_tape(&[0xC0], 128);
        let steps = Skim::execute(&mut tape, 8192);
        assert_eq!(steps, 1);
    }

    #[test]
    fn test_inc_then_halt() {
        // 0x30 = INC skip 1, 0xC0 = HALT.
        // pc=0: INC, acc=1, pc=1. pc=1: HALT.
        let mut tape = make_tape(&[0x30, 0xC0], 128);
        let steps = Skim::execute(&mut tape, 8192);
        assert_eq!(steps, 2);
    }

    #[test]
    fn test_skip_distance() {
        // 0x33 = INC skip 4. Next instruction at pc=4.
        // 0xC0 at position 4 = HALT.
        // Positions 1,2,3 should be skipped.
        let mut tape = make_tape(&[0x33, 0x30, 0x30, 0x30, 0xC0], 128);
        let steps = Skim::execute(&mut tape, 8192);
        assert_eq!(steps, 2); // INC at 0, HALT at 4
    }

    #[test]
    fn test_store_to_wp() {
        // INC (acc=1), INC (acc=2), STORE (tape[64]=2), HALT.
        // All with skip 1 (low nibble 0).
        let mut tape = make_tape(&[0x30, 0x30, 0x10, 0xC0], 128);
        Skim::execute(&mut tape, 8192);
        assert_eq!(tape[64], 2);
    }

    #[test]
    fn test_load_from_wp() {
        // LOAD (acc=tape[64]), STORE at wp+1... actually just check acc via store.
        // Set tape[64] = 42. LOAD (acc=42). WP_INC (wp=65). STORE (tape[65]=42). HALT.
        let mut tape = make_tape(&[0x00, 0x60, 0x10, 0xC0], 128);
        tape[64] = 42;
        Skim::execute(&mut tape, 8192);
        assert_eq!(tape[65], 42);
    }

    #[test]
    fn test_copy_fwd() {
        // COPY_FWD at pc=0: tape[64] = tape[0] = 0x20; wp becomes 65.
        let mut tape = make_tape(&[0x20, 0xC0], 128);
        Skim::execute(&mut tape, 8192);
        assert_eq!(tape[64], 0x20); // copied itself
    }

    #[test]
    fn test_copy_fwd_advances_wp() {
        // Two COPY_FWDs: first copies tape[0] to tape[64], second copies tape[1] to tape[65].
        let mut tape = make_tape(&[0x20, 0x20, 0xC0], 128);
        Skim::execute(&mut tape, 8192);
        assert_eq!(tape[64], 0x20);
        assert_eq!(tape[65], 0x20);
    }

    #[test]
    fn test_dec() {
        // DEC from 0 wraps to 255. STORE. HALT.
        let mut tape = make_tape(&[0x40, 0x10, 0xC0], 128);
        Skim::execute(&mut tape, 8192);
        assert_eq!(tape[64], 255);
    }

    #[test]
    fn test_xor() {
        // Set tape[64] = 0xFF. INC (acc=1). XOR (acc = 1 ^ 0xFF = 0xFE). STORE at wp=65 after WP_INC. HALT.
        let mut tape = make_tape(&[0x30, 0x50, 0x60, 0x10, 0xC0], 128);
        tape[64] = 0xFF;
        Skim::execute(&mut tape, 8192);
        assert_eq!(tape[65], 0xFE);
    }

    #[test]
    fn test_wp_inc_dec() {
        // WP_INC twice (wp=66), WP_DEC once (wp=65), STORE (tape[65]=0). HALT.
        let mut tape = make_tape(&[0x60, 0x60, 0x70, 0x10, 0xC0], 128);
        Skim::execute(&mut tape, 8192);
        // acc=0, so tape[65] = 0.
        assert_eq!(tape[65], 0);
    }

    #[test]
    fn test_set_wp() {
        // INC 100 times... too many. Instead: use a different approach.
        // INC (acc=1), SET_WP (wp=1), STORE (tape[1]=1). HALT.
        let mut tape = make_tape(&[0x30, 0x80, 0x10, 0xC0], 128);
        Skim::execute(&mut tape, 8192);
        assert_eq!(tape[1], 1);
    }

    #[test]
    fn test_get_wp() {
        // GET_WP (acc=64), STORE (tape[64]=64). HALT.
        let mut tape = make_tape(&[0x90, 0x10, 0xC0], 128);
        Skim::execute(&mut tape, 8192);
        assert_eq!(tape[64], 64);
    }

    #[test]
    fn test_skz_taken() {
        // acc=0, SKZ with skip 3 (0xA2): skip to pc+3. Put HALT there.
        // If taken, goes to pc=3 (skipping positions 1,2).
        let mut tape = make_tape(&[0xA2, 0x30, 0x30, 0xC0], 128);
        let steps = Skim::execute(&mut tape, 8192);
        assert_eq!(steps, 2); // SKZ at 0 -> HALT at 3
    }

    #[test]
    fn test_skz_not_taken() {
        // acc=1, SKZ: skip 1 (go to next byte).
        let mut tape = make_tape(&[0x30, 0xA2, 0xC0, 0x30, 0xC0], 128);
        // INC(acc=1) at 0 -> SKZ at 1 (acc!=0, skip 1 -> pc=2) -> HALT at 2.
        let steps = Skim::execute(&mut tape, 8192);
        assert_eq!(steps, 3);
    }

    #[test]
    fn test_sknz_taken() {
        // INC (acc=1), SKNZ skip 2 (0xB1): acc!=0, take the skip.
        // pc=0: INC skip 1 -> pc=1. pc=1: SKNZ(acc=1!=0) skip 2 -> pc=3.
        // pc=3: 0x30=INC skip 1 -> pc=4. pc=4: HALT.
        let mut tape = make_tape(&[0x30, 0xB1, 0x30, 0x30, 0xC0], 128);
        let steps = Skim::execute(&mut tape, 8192);
        assert_eq!(steps, 4);
    }

    #[test]
    fn test_sknz_not_taken() {
        // acc=0, SKNZ: acc==0, skip 1 -> next byte.
        let mut tape = make_tape(&[0xB2, 0xC0, 0x30, 0xC0], 128);
        // SKNZ at 0: acc=0, skip 1 -> pc=1. HALT at 1.
        let steps = Skim::execute(&mut tape, 8192);
        assert_eq!(steps, 2);
    }

    #[test]
    fn test_pc_wraps() {
        // Put INC with skip 0xF (skip 16) at position 120.
        // Next pc = (120 + 16) % 128 = 8. Put HALT at position 8.
        let mut tape = vec![0xD0; 128]; // all NOPs with skip 1
        tape[0] = 0xDF; // NOP skip 16 -> pc = 16
        tape[16] = 0xDF; // NOP skip 16 -> pc = 32
        tape[32] = 0xDF; // NOP skip 16 -> pc = 48
        tape[48] = 0xDF; // NOP skip 16 -> pc = 64
        tape[64] = 0xDF; // NOP skip 16 -> pc = 80
        tape[80] = 0xDF; // NOP skip 16 -> pc = 96
        tape[96] = 0xDF; // NOP skip 16 -> pc = 112
        tape[112] = 0xDF; // NOP skip 16 -> pc = 0 (wrap!)
        // Now at pc=0 again which is 0xDF -> infinite loop. Use step limit.
        let steps = Skim::execute(&mut tape, 20);
        assert_eq!(steps, 20);
    }

    #[test]
    fn test_nop_still_skips() {
        // NOPs (0xD_, 0xE_, 0xF_) still use their low nibble for skip distance.
        // 0xD3 = NOP skip 4. At pc=0, next pc = 4.
        let mut tape = make_tape(&[0xD3, 0x30, 0x30, 0x30, 0xC0], 128);
        let steps = Skim::execute(&mut tape, 8192);
        assert_eq!(steps, 2); // NOP at 0, HALT at 4
    }

    #[test]
    fn test_empty_tape() {
        let mut tape: Vec<u8> = vec![];
        let steps = Skim::execute(&mut tape, 8192);
        assert_eq!(steps, 0);
    }

    #[test]
    fn test_step_limit() {
        // INC with skip 1 at every position: infinite loop through tape.
        let mut tape = vec![0x30; 128];
        let steps = Skim::execute(&mut tape, 100);
        assert_eq!(steps, 100);
    }

    // --- Replicator tests ---

    #[test]
    fn test_64_byte_copy_fwd_replicator() {
        // 64 copies of COPY_FWD with skip 1 (0x20).
        // Each copies tape[pc] to tape[wp], advances wp.
        // Visits pc=0,1,...,63 copying each to wp=64,65,...,127.
        let mut tape = vec![0x20; 64];
        tape.extend(vec![0u8; 64]);

        Skim::execute(&mut tape, 8192);

        assert_eq!(&tape[64..128], &vec![0x20u8; 64]);
    }

    #[test]
    fn test_64_byte_replicator_fixed_point() {
        let mut tape1 = vec![0x20; 64];
        tape1.extend(vec![0u8; 64]);
        Skim::execute(&mut tape1, 8192);
        let copy1 = tape1[64..128].to_vec();

        let mut tape2 = vec![0u8; 128];
        tape2[..64].copy_from_slice(&copy1);
        Skim::execute(&mut tape2, 8192);
        let copy2 = tape2[64..128].to_vec();

        assert_eq!(copy1, copy2, "Skim replicator should be a fixed point");
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
            let steps = Skim::execute(&mut tape, 8192);
            prop_assert!(steps <= 8192);
        }

        #[test]
        fn random_programs_respect_step_limit(
            tape_data in prop::collection::vec(any::<u8>(), 1..256),
            limit in 1usize..1000
        ) {
            let mut tape = tape_data;
            let steps = Skim::execute(&mut tape, limit);
            prop_assert!(steps <= limit);
        }

        #[test]
        fn output_tape_same_length(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let original_len = tape_data.len();
            let mut tape = tape_data;
            Skim::execute(&mut tape, 8192);
            prop_assert_eq!(tape.len(), original_len);
        }
    }
}
