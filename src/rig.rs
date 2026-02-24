use crate::substrate::Substrate;

/// The Rig (Register-Indirect Goto) instruction set — a register machine substrate.
///
/// A classical Von Neumann architecture with a small register file and
/// register-indirect addressing. This covers the one major computational
/// paradigm missing from the other substrates: named registers with
/// indirect memory access.
///
/// State:
/// - `pc`: instruction pointer, starts at 0
/// - `r[0]`: general register, starts at 0
/// - `r[1]`: general register, starts at `tape.len() / 2`
/// - `r[2]`: general register, starts at 0
/// - `r[3]`: general register, starts at 0
///
/// Instruction encoding (1 byte):
/// - High 4 bits: opcode
/// - Low 4 bits: two 2-bit register selectors — bits[3:2] = dst, bits[1:0] = src
///
/// All arithmetic wraps modulo 256 (u8). All addresses wrap modulo tape length.
pub struct Rig;

struct RigState {
    pc: usize,
    r: [u8; 4],
}

/// Execute one Rig instruction. Returns true if still running.
fn rig_step(state: &mut RigState, tape: &mut [u8]) -> bool {
    let len = tape.len();
    if state.pc >= len {
        return false;
    }
    let instr = tape[state.pc];
    let dst = ((instr >> 2) & 0x03) as usize;
    let src = (instr & 0x03) as usize;

    match instr >> 4 {
        0x0 => {
            // LOAD: r[dst] = tape[r[src]]
            state.r[dst] = tape[state.r[src] as usize % len];
        }
        0x1 => {
            // STORE: tape[r[dst]] = r[src]
            tape[state.r[dst] as usize % len] = state.r[src];
        }
        0x2 => {
            // MOV: r[dst] = r[src]
            state.r[dst] = state.r[src];
        }
        0x3 => {
            // ADD: r[dst] += r[src]
            state.r[dst] = state.r[dst].wrapping_add(state.r[src]);
        }
        0x4 => {
            // SUB: r[dst] -= r[src]
            state.r[dst] = state.r[dst].wrapping_sub(state.r[src]);
        }
        0x5 => {
            // XOR: r[dst] ^= r[src]
            state.r[dst] ^= state.r[src];
        }
        0x6 => {
            // INC: r[dst]++ (src ignored)
            state.r[dst] = state.r[dst].wrapping_add(1);
        }
        0x7 => {
            // DEC: r[dst]-- (src ignored)
            state.r[dst] = state.r[dst].wrapping_sub(1);
        }
        0x8 => {
            // JZ: if r[src] == 0, pc = r[dst] as usize
            if state.r[src] == 0 {
                state.pc = state.r[dst] as usize;
                return true;
            }
        }
        0x9 => {
            // JNZ: if r[src] != 0, pc = r[dst] as usize
            if state.r[src] != 0 {
                state.pc = state.r[dst] as usize;
                return true;
            }
        }
        0xA => {
            // COPY: tape[r[dst]] = tape[r[src]]
            let s = state.r[src] as usize % len;
            let d = state.r[dst] as usize % len;
            tape[d] = tape[s];
        }
        0xB => {
            // HALT
            return false;
        }
        // 0xC-0xF: NOP
        _ => {}
    }

    state.pc += 1;
    true
}

impl Substrate for Rig {
    fn execute(tape: &mut [u8], step_limit: usize) -> usize {
        let len = tape.len();
        if len == 0 {
            return 0;
        }

        let mut state = RigState {
            pc: 0,
            r: [0, (len / 2) as u8, 0, 0],
        };
        let mut steps = 0;

        while state.pc < len && steps < step_limit {
            steps += 1;
            if !rig_step(&mut state, tape) {
                break;
            }
        }

        steps
    }

    fn execute_battle(tape: &mut [u8], ps: usize, step_limit: usize) -> usize {
        let len = tape.len();
        if len == 0 {
            return 0;
        }
        let mut a = RigState {
            pc: 0,
            r: [0, ps as u8, 0, 0],
        };
        let mut b = RigState {
            pc: ps,
            r: [ps as u8, 0, 0, 0],
        };
        let mut steps = 0;
        let mut halted_a = false;
        let mut halted_b = false;
        while steps < step_limit && (!halted_a || !halted_b) {
            if !halted_a {
                halted_a = !rig_step(&mut a, tape);
                steps += 1;
                if steps >= step_limit {
                    break;
                }
            }
            if !halted_b {
                halted_b = !rig_step(&mut b, tape);
                steps += 1;
            }
        }
        steps
    }

    fn is_instruction(byte: u8) -> bool {
        (byte >> 4) <= 0xB
    }

    fn disassemble(tape: &[u8]) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        let reg_name = |i: usize| -> &'static str {
            match i {
                0 => "r0",
                1 => "r1",
                2 => "r2",
                3 => "r3",
                _ => "r?",
            }
        };
        for (addr, &b) in tape.iter().enumerate() {
            let dst = ((b >> 2) & 0x03) as usize;
            let src = (b & 0x03) as usize;
            let desc = match b >> 4 {
                0x0 => format!("LOAD {}, [{}]", reg_name(dst), reg_name(src)),
                0x1 => format!("STORE [{}], {}", reg_name(dst), reg_name(src)),
                0x2 => format!("MOV {}, {}", reg_name(dst), reg_name(src)),
                0x3 => format!("ADD {}, {}", reg_name(dst), reg_name(src)),
                0x4 => format!("SUB {}, {}", reg_name(dst), reg_name(src)),
                0x5 => format!("XOR {}, {}", reg_name(dst), reg_name(src)),
                0x6 => format!("INC {}", reg_name(dst)),
                0x7 => format!("DEC {}", reg_name(dst)),
                0x8 => format!("JZ {}, {}", reg_name(dst), reg_name(src)),
                0x9 => format!("JNZ {}, {}", reg_name(dst), reg_name(src)),
                0xA => format!("COPY [{}], [{}]", reg_name(dst), reg_name(src)),
                0xB => "HALT".to_string(),
                _ => "NOP".to_string(),
            };
            let _ = writeln!(out, "{addr:04X}: {b:02X}  {desc}");
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

    // Register encoding helpers
    // dst=r0 src=r0 -> 0x00, dst=r1 src=r0 -> 0x04
    // dst=r0 src=r1 -> 0x01, dst=r1 src=r1 -> 0x05
    // dst bits[3:2], src bits[1:0]

    #[test]
    fn test_halt() {
        let mut tape = make_tape(&[0xB0], 128);
        let steps = Rig::execute(&mut tape, 8192);
        assert_eq!(steps, 1);
    }

    #[test]
    fn test_inc_r0() {
        // INC r0 (0x60), INC r0 (0x60), STORE [r1], r0 (0x10 + dst=r1,src=r0 = 0x14), HALT
        // r0 becomes 2, stored to tape[r1=64].
        let mut tape = make_tape(&[0x60, 0x60, 0x14, 0xB0], 128);
        Rig::execute(&mut tape, 8192);
        assert_eq!(tape[64], 2);
    }

    #[test]
    fn test_dec_r0_wraps() {
        // DEC r0 (0x70): 0 - 1 = 255. STORE [r1], r0. HALT.
        let mut tape = make_tape(&[0x70, 0x14, 0xB0], 128);
        Rig::execute(&mut tape, 8192);
        assert_eq!(tape[64], 255);
    }

    #[test]
    fn test_load() {
        // Set tape[64] = 42. LOAD r0, [r1] (0x01): r0 = tape[r1=64] = 42.
        // STORE [r1], r0: tape[64] = 42 (unchanged, but confirms load worked).
        // INC r1, STORE [r1], r0: tape[65] = 42.
        let mut tape = make_tape(&[0x01, 0x64, 0x14, 0xB0], 128);
        tape[64] = 42;
        Rig::execute(&mut tape, 8192);
        assert_eq!(tape[65], 42);
    }

    #[test]
    fn test_store() {
        // INC r0 x3 -> r0=3. STORE [r1], r0: tape[64] = 3.
        let mut tape = make_tape(&[0x60, 0x60, 0x60, 0x14, 0xB0], 128);
        Rig::execute(&mut tape, 8192);
        assert_eq!(tape[64], 3);
    }

    #[test]
    fn test_mov() {
        // MOV r2, r1 (0x29): r2 = r1 = 64. STORE [r1], r2 (0x16): tape[64] = 64.
        // 0x16: STORE, dst=r1(01), src=r2(10) -> low nibble = 0110 = 0x6.
        let mut tape = make_tape(&[0x29, 0x16, 0xB0], 128);
        Rig::execute(&mut tape, 8192);
        assert_eq!(tape[64], 64);
    }

    #[test]
    fn test_add() {
        // INC r0 (r0=1). INC r0 (r0=2). ADD r0, r1 (0x31): r0 = 2 + 64 = 66.
        // STORE [r1], r0: tape[64] = 66.
        let mut tape = make_tape(&[0x60, 0x60, 0x31, 0x14, 0xB0], 128);
        Rig::execute(&mut tape, 8192);
        assert_eq!(tape[64], 66);
    }

    #[test]
    fn test_sub() {
        // MOV r0, r1 (0x01... no, MOV is 0x2_). MOV r0, r1 = 0x21: r0 = r1 = 64.
        // INC r0 (r0=65). SUB r0, r1 (0x41): r0 = 65 - 64 = 1.
        // STORE [r1], r0: tape[64] = 1.
        let mut tape = make_tape(&[0x21, 0x60, 0x41, 0x14, 0xB0], 128);
        Rig::execute(&mut tape, 8192);
        assert_eq!(tape[64], 1);
    }

    #[test]
    fn test_xor() {
        // INC r0 x3 (r0=3). MOV r2, r1 (r2=64).
        // XOR r0, r2 (0x52): r0 = 3 ^ 64 = 67. STORE [r1], r0: tape[64] = 67.
        let mut tape = make_tape(&[0x60, 0x60, 0x60, 0x29, 0x52, 0x14, 0xB0], 128);
        Rig::execute(&mut tape, 8192);
        assert_eq!(tape[64], 67);
    }

    #[test]
    fn test_jz_taken() {
        // r0=0, r3=0. JZ r3, r0 (0x8C): r0==0, pc = r3 = 0. Infinite loop.
        let mut tape = make_tape(&[0x8C, 0xB0], 128);
        let steps = Rig::execute(&mut tape, 10);
        assert_eq!(steps, 10);
    }

    #[test]
    fn test_jz_not_taken() {
        // INC r0 (r0=1). JZ r3, r0 (0x8C): r0!=0, fall through. HALT.
        let mut tape = make_tape(&[0x60, 0x8C, 0xB0], 128);
        let steps = Rig::execute(&mut tape, 8192);
        assert_eq!(steps, 3);
    }

    #[test]
    fn test_jnz_taken() {
        // INC r0 (r0=1). JNZ r3, r0 (0x9C): r0!=0, pc = r3 = 0. Loop.
        let mut tape = make_tape(&[0x60, 0x9C], 128);
        let steps = Rig::execute(&mut tape, 10);
        assert_eq!(steps, 10);
    }

    #[test]
    fn test_jnz_not_taken() {
        // r0=0, r3=0. JNZ r3, r0 (0x9C): r0==0, fall through. HALT.
        let mut tape = make_tape(&[0x9C, 0xB0], 128);
        let steps = Rig::execute(&mut tape, 8192);
        assert_eq!(steps, 2);
    }

    #[test]
    fn test_copy_instruction() {
        // COPY [r1], [r0] (0xA4): tape[r1=64] = tape[r0=0].
        // tape[0] = 0xA4 (the COPY instruction itself).
        let mut tape = make_tape(&[0xA4, 0xB0], 128);
        Rig::execute(&mut tape, 8192);
        assert_eq!(tape[64], 0xA4);
    }

    #[test]
    fn test_r1_starts_at_half() {
        // STORE [r1], r1 (0x15): tape[r1] = r1. In a 128-byte tape, r1=64.
        let mut tape = make_tape(&[0x15, 0xB0], 128);
        Rig::execute(&mut tape, 8192);
        assert_eq!(tape[64], 64);
    }

    #[test]
    fn test_nop_bytes() {
        // 0xC0-0xFF are NOPs. Should just advance pc.
        let mut tape = make_tape(&[0xC0, 0xD5, 0xEF, 0xFF, 0x60, 0x14, 0xB0], 128);
        Rig::execute(&mut tape, 8192);
        assert_eq!(tape[64], 1); // only one INC executed
    }

    #[test]
    fn test_empty_tape() {
        let mut tape: Vec<u8> = vec![];
        let steps = Rig::execute(&mut tape, 8192);
        assert_eq!(steps, 0);
    }

    #[test]
    fn test_step_limit() {
        // JZ r3, r3 at pc=0: r3==0, pc = r3 = 0. Infinite loop.
        let mut tape = make_tape(&[0x8F], 128);
        let steps = Rig::execute(&mut tape, 100);
        assert_eq!(steps, 100);
    }

    // --- Replicator tests ---

    #[test]
    fn test_4_byte_self_replicator() {
        // COPY [r1], [r0] (0xA4): tape[r1] = tape[r0]
        // INC r0 (0x60): r0++
        // INC r1 (0x64): r1++
        // JNZ r3, r0 (0x9C): if r0 != 0, pc = r3 = 0
        //
        // r0 starts at 0, r1 at 64, r3 at 0 (jump target).
        // Copies tape[0..255] to tape[64..] wrapping, but after 64 iterations
        // all of the first half is copied to the second half.
        let replicator: [u8; 4] = [0xA4, 0x60, 0x64, 0x9C];

        let mut tape = vec![0u8; 128];
        tape[..4].copy_from_slice(&replicator);

        Rig::execute(&mut tape, 8192);

        // First 4 bytes of second half should match the replicator.
        assert_eq!(&tape[64..68], &replicator);
        // Rest should be zeros (copied from the zero-padded first half).
        assert_eq!(&tape[68..128], &vec![0u8; 60]);
    }

    #[test]
    fn test_replicator_is_functional_fixed_point() {
        let replicator: [u8; 4] = [0xA4, 0x60, 0x64, 0x9C];

        // Run 1: original replicates.
        let mut tape1 = vec![0u8; 128];
        tape1[..4].copy_from_slice(&replicator);
        Rig::execute(&mut tape1, 8192);
        let copy1 = tape1[64..128].to_vec();

        // Run 2: the copy should also replicate.
        let mut tape2 = vec![0u8; 128];
        tape2[..64].copy_from_slice(&copy1);
        Rig::execute(&mut tape2, 8192);
        let copy2 = tape2[64..128].to_vec();

        assert_eq!(copy1, copy2, "Rig replicator should be a fixed point");
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
            let steps = Rig::execute(&mut tape, 8192);
            prop_assert!(steps <= 8192);
        }

        #[test]
        fn random_programs_respect_step_limit(
            tape_data in prop::collection::vec(any::<u8>(), 1..256),
            limit in 1usize..1000
        ) {
            let mut tape = tape_data;
            let steps = Rig::execute(&mut tape, limit);
            prop_assert!(steps <= limit);
        }

        #[test]
        fn output_tape_same_length(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let original_len = tape_data.len();
            let mut tape = tape_data;
            Rig::execute(&mut tape, 8192);
            prop_assert_eq!(tape.len(), original_len);
        }
    }
}
