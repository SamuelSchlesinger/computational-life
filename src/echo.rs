use crate::substrate::Substrate;

/// The Echo (Delay-Line Memory) substrate.
///
/// Inspired by mercury delay-line memory (EDSAC, UNIVAC I). The write address
/// is always constrained to `read_pointer + delay` — there is no random-access
/// write capability. All data movement is mediated through the delay offset.
///
/// State:
/// - `pc`: instruction pointer, starts at 0
/// - `rp`: read pointer, starts at 0
/// - `delay`: offset register (u8), initialized to `tape.len() / 2`
/// - `acc`: accumulator (u8), starts at 0
///
/// The write pointer (wp) is always `(rp + delay) % tape.len()`.
pub struct Echo;

// Opcodes
const HALT: u8 = 0x00;
const ECHO: u8 = 0x01;
const LOAD: u8 = 0x02;
const STORE: u8 = 0x03;
const SKIP: u8 = 0x04;
const SET_DELAY: u8 = 0x05;
const INC: u8 = 0x06;
const DEC: u8 = 0x07;
const XOR: u8 = 0x08;
const ADD: u8 = 0x09;
const JMP_REL: u8 = 0x0A;
const JZ: u8 = 0x0B;
const JNZ: u8 = 0x0C;
const SKIP_EQ: u8 = 0x0D;
const GET_DELAY: u8 = 0x0E;
const SET_RP: u8 = 0x0F;

struct EchoState {
    pc: usize,
    rp: u8,
    delay: u8,
    acc: u8,
}

/// Execute one Echo instruction. Returns true if still running.
fn echo_step(state: &mut EchoState, tape: &mut [u8]) -> bool {
    let len = tape.len();
    if state.pc >= len {
        return false;
    }
    let wp = state.rp.wrapping_add(state.delay);

    match tape[state.pc] {
        HALT => return false,
        ECHO => {
            let src = state.rp as usize % len;
            let dst = wp as usize % len;
            tape[dst] = tape[src];
            state.rp = state.rp.wrapping_add(1);
        }
        LOAD => {
            state.acc = tape[state.rp as usize % len];
            state.rp = state.rp.wrapping_add(1);
        }
        STORE => {
            tape[wp as usize % len] = state.acc;
        }
        SKIP => {
            state.rp = state.rp.wrapping_add(1);
        }
        SET_DELAY => {
            if state.pc + 1 >= len {
                return false;
            }
            state.delay = tape[state.pc + 1];
            state.pc += 2;
            return true;
        }
        INC => {
            state.acc = state.acc.wrapping_add(1);
        }
        DEC => {
            state.acc = state.acc.wrapping_sub(1);
        }
        XOR => {
            state.acc ^= tape[state.rp as usize % len];
        }
        ADD => {
            state.acc = state.acc.wrapping_add(tape[state.rp as usize % len]);
            state.rp = state.rp.wrapping_add(1);
        }
        JMP_REL => {
            if state.pc + 1 >= len {
                return false;
            }
            let offset = tape[state.pc + 1] as i8;
            let new_pc = state.pc as isize + 2 + offset as isize;
            if new_pc < 0 {
                return false;
            }
            state.pc = new_pc as usize;
            return true;
        }
        JZ => {
            if state.pc + 1 >= len {
                return false;
            }
            if state.acc == 0 {
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
        JNZ => {
            if state.pc + 1 >= len {
                return false;
            }
            if state.acc != 0 {
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
        SKIP_EQ => {
            let r = tape[state.rp as usize % len];
            let w = tape[wp as usize % len];
            if r == w {
                state.pc += 2;
                return true;
            }
        }
        GET_DELAY => {
            state.acc = state.delay;
        }
        SET_RP => {
            state.rp = state.acc;
        }
        _ => {} // NOP (0x10-0xFF)
    }
    state.pc += 1;
    true
}

impl Substrate for Echo {
    fn execute(tape: &mut [u8], step_limit: usize) -> usize {
        let len = tape.len();
        if len == 0 {
            return 0;
        }

        let mut state = EchoState {
            pc: 0,
            rp: 0,
            delay: (len / 2) as u8,
            acc: 0,
        };
        let mut steps = 0;

        while state.pc < len && steps < step_limit {
            steps += 1;
            if !echo_step(&mut state, tape) {
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
        let mut a = EchoState {
            pc: 0,
            rp: 0,
            delay: ps as u8,
            acc: 0,
        };
        let mut b = EchoState {
            pc: ps,
            rp: ps as u8,
            delay: ps as u8,
            acc: 0,
        };
        let mut steps = 0;
        let mut halted_a = false;
        let mut halted_b = false;
        while steps < step_limit && (!halted_a || !halted_b) {
            if !halted_a {
                halted_a = !echo_step(&mut a, tape);
                steps += 1;
                if steps >= step_limit {
                    break;
                }
            }
            if !halted_b {
                halted_b = !echo_step(&mut b, tape);
                steps += 1;
            }
        }
        steps
    }

    fn is_instruction(byte: u8) -> bool {
        byte <= SET_RP
    }

    fn disassemble(tape: &[u8]) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        let mut pc = 0;
        while pc < tape.len() {
            let b = tape[pc];
            let addr = pc;
            let desc = match b {
                HALT => {
                    pc += 1;
                    "HALT".to_string()
                }
                ECHO => {
                    pc += 1;
                    "ECHO".to_string()
                }
                LOAD => {
                    pc += 1;
                    "LOAD".to_string()
                }
                STORE => {
                    pc += 1;
                    "STORE".to_string()
                }
                SKIP => {
                    pc += 1;
                    "SKIP".to_string()
                }
                SET_DELAY => {
                    if pc + 1 < tape.len() {
                        let val = tape[pc + 1];
                        pc += 2;
                        format!("SET_DELAY {val}")
                    } else {
                        pc += 1;
                        "SET_DELAY ???".to_string()
                    }
                }
                INC => {
                    pc += 1;
                    "INC".to_string()
                }
                DEC => {
                    pc += 1;
                    "DEC".to_string()
                }
                XOR => {
                    pc += 1;
                    "XOR".to_string()
                }
                ADD => {
                    pc += 1;
                    "ADD".to_string()
                }
                JMP_REL => {
                    if pc + 1 < tape.len() {
                        let offset = tape[pc + 1] as i8;
                        let target = pc as isize + 2 + offset as isize;
                        pc += 2;
                        format!("JMP_REL {offset:+} (-> {target})")
                    } else {
                        pc += 1;
                        "JMP_REL ???".to_string()
                    }
                }
                JZ => {
                    if pc + 1 < tape.len() {
                        let offset = tape[pc + 1] as i8;
                        let target = pc as isize + 2 + offset as isize;
                        pc += 2;
                        format!("JZ {offset:+} (-> {target})")
                    } else {
                        pc += 1;
                        "JZ ???".to_string()
                    }
                }
                JNZ => {
                    if pc + 1 < tape.len() {
                        let offset = tape[pc + 1] as i8;
                        let target = pc as isize + 2 + offset as isize;
                        pc += 2;
                        format!("JNZ {offset:+} (-> {target})")
                    } else {
                        pc += 1;
                        "JNZ ???".to_string()
                    }
                }
                SKIP_EQ => {
                    pc += 1;
                    "SKIP_EQ".to_string()
                }
                GET_DELAY => {
                    pc += 1;
                    "GET_DELAY".to_string()
                }
                SET_RP => {
                    pc += 1;
                    "SET_RP".to_string()
                }
                _ => {
                    pc += 1;
                    "NOP".to_string()
                }
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

    #[test]
    fn test_empty_tape() {
        let mut tape: Vec<u8> = vec![];
        let steps = Echo::execute(&mut tape, 8192);
        assert_eq!(steps, 0);
    }

    #[test]
    fn test_halt() {
        let mut tape = make_tape(&[HALT, INC, INC], 128);
        let steps = Echo::execute(&mut tape, 8192);
        assert_eq!(steps, 1);
    }

    #[test]
    fn test_echo_copies_at_delay() {
        // Default delay = 64 (tape_len/2). ECHO copies tape[rp=0] to tape[0+64=64].
        let mut tape = make_tape(&[ECHO], 128);
        Echo::execute(&mut tape, 8192);
        assert_eq!(tape[64], ECHO); // copied the ECHO instruction itself
    }

    #[test]
    fn test_echo_advances_rp() {
        // ECHO, ECHO: first copies tape[0] to tape[64], rp becomes 1.
        // Second copies tape[1] to tape[65].
        let mut tape = make_tape(&[ECHO, ECHO], 128);
        tape[1] = ECHO; // will be copied
        Echo::execute(&mut tape, 8192);
        assert_eq!(tape[64], ECHO);
        assert_eq!(tape[65], ECHO);
    }

    #[test]
    fn test_set_delay() {
        // SET_DELAY 10: delay becomes 10. Then ECHO copies tape[rp=0] to tape[0+10=10].
        // rp=0 → tape[0] = SET_DELAY = 0x05
        let mut tape = make_tape(&[SET_DELAY, 10, ECHO], 128);
        Echo::execute(&mut tape, 8192);
        assert_eq!(tape[10], SET_DELAY); // rp was 0 when ECHO ran, copies tape[0]
    }

    #[test]
    fn test_load_and_store() {
        // LOAD reads tape[rp=0] into acc, advances rp.
        // STORE writes acc to tape[wp].
        let mut tape = make_tape(&[LOAD, STORE], 128);
        tape[0] = LOAD; // acc = 0x02 (LOAD opcode)
        Echo::execute(&mut tape, 8192);
        // wp = rp(1) + delay(64) = 65 when STORE executes
        assert_eq!(tape[65], LOAD);
    }

    #[test]
    fn test_skip_advances_rp() {
        // SKIP x3 advances rp to 3. GET_DELAY reads delay (64) into acc. STORE writes acc.
        // After 3 SKIPs, rp=3, so wp = 3+64 = 67.
        let mut tape = make_tape(&[SKIP, SKIP, SKIP, GET_DELAY, STORE], 128);
        Echo::execute(&mut tape, 8192);
        // acc = 64 (delay), stored at tape[wp] where wp = rp(3)+delay(64) = 67
        assert_eq!(tape[67], 64);
    }

    #[test]
    fn test_inc_dec() {
        // INC x3, DEC x1 → acc=2. STORE writes to tape[wp].
        let mut tape = make_tape(&[INC, INC, INC, DEC, STORE], 128);
        Echo::execute(&mut tape, 8192);
        // wp = rp(0) + delay(64) = 64
        assert_eq!(tape[64], 2);
    }

    #[test]
    fn test_xor() {
        // tape[0] = XOR = 0x08. XOR: acc ^= tape[rp=0] = 0x08. acc = 0 ^ 0x08 = 0x08.
        let mut tape = make_tape(&[XOR, STORE], 128);
        Echo::execute(&mut tape, 8192);
        assert_eq!(tape[64], XOR);
    }

    #[test]
    fn test_add() {
        // tape[0] = ADD = 0x09. ADD: acc += tape[rp=0] = 0x09. acc = 9. rp advances.
        let mut tape = make_tape(&[ADD, STORE], 128);
        Echo::execute(&mut tape, 8192);
        // wp = rp(1) + delay(64) = 65
        assert_eq!(tape[65], ADD);
    }

    #[test]
    fn test_jmp_rel() {
        // JMP_REL +1 skips one byte.
        let mut tape = make_tape(&[JMP_REL, 0x01, INC, STORE], 128);
        Echo::execute(&mut tape, 8192);
        assert_eq!(tape[64], 0); // INC was skipped, acc stayed 0
    }

    #[test]
    fn test_jz_taken() {
        // acc=0. JZ +1 → skips INC.
        let mut tape = make_tape(&[JZ, 0x01, INC, STORE], 128);
        Echo::execute(&mut tape, 8192);
        assert_eq!(tape[64], 0);
    }

    #[test]
    fn test_jz_not_taken() {
        let mut tape = make_tape(&[INC, JZ, 0x01, INC, STORE], 128);
        Echo::execute(&mut tape, 8192);
        assert_eq!(tape[64], 2); // both INCs executed
    }

    #[test]
    fn test_jnz_taken() {
        let mut tape = make_tape(&[INC, JNZ, 0x01, INC, STORE], 128);
        Echo::execute(&mut tape, 8192);
        assert_eq!(tape[64], 1); // second INC skipped
    }

    #[test]
    fn test_jnz_not_taken() {
        let mut tape = make_tape(&[JNZ, 0x01, INC, STORE], 128);
        Echo::execute(&mut tape, 8192);
        assert_eq!(tape[64], 1); // INC executed
    }

    #[test]
    fn test_skip_eq() {
        // rp=0, wp=rp+delay=64. If tape[0] == tape[64], skip next instruction.
        let mut tape = make_tape(&[SKIP_EQ, INC, STORE], 128);
        // tape[0] = SKIP_EQ = 0x0D, tape[64] = 0x00 → not equal → don't skip
        Echo::execute(&mut tape, 8192);
        assert_eq!(tape[64], 1); // INC executed

        // Now make them equal
        let mut tape2 = make_tape(&[SKIP_EQ, INC, STORE], 128);
        tape2[64] = SKIP_EQ; // make tape[0] == tape[64]
        Echo::execute(&mut tape2, 8192);
        // SKIP_EQ: tape[0]=0x0D, tape[64]=0x0D → equal → skip INC
        // acc stays 0. STORE writes 0 to tape[wp].
        assert_eq!(tape2[64], 0); // INC was skipped, STORE wrote 0
    }

    #[test]
    fn test_get_delay() {
        let mut tape = make_tape(&[GET_DELAY, STORE], 128);
        Echo::execute(&mut tape, 8192);
        assert_eq!(tape[64], 64); // delay defaults to tape_len/2 = 64
    }

    #[test]
    fn test_set_rp() {
        // INC x10 → acc=10. SET_RP: rp=10. STORE writes acc to tape[rp+delay]=tape[74].
        let mut program: Vec<u8> = vec![INC; 10];
        program.extend_from_slice(&[SET_RP, STORE]);
        let mut tape = make_tape(&program, 128);
        Echo::execute(&mut tape, 8192);
        // wp = rp(10) + delay(64) = 74
        assert_eq!(tape[74], 10);
    }

    #[test]
    fn test_nop_bytes() {
        let mut tape = make_tape(&[0x10, 0x80, 0xFF, INC, STORE], 128);
        Echo::execute(&mut tape, 8192);
        assert_eq!(tape[64], 1); // only one INC
    }

    #[test]
    fn test_step_limit() {
        // Infinite loop: JMP_REL -2
        let mut tape = make_tape(&[JMP_REL, 0xFE], 128);
        let steps = Echo::execute(&mut tape, 100);
        assert_eq!(steps, 100);
    }

    #[test]
    fn test_negative_jump_terminates() {
        let mut tape = make_tape(&[JMP_REL, 0x80], 128); // offset -128
        let steps = Echo::execute(&mut tape, 8192);
        assert_eq!(steps, 1);
    }

    // --- Replicator tests ---

    #[test]
    fn test_3_byte_self_replicator() {
        // [ECHO, JMP_REL, 0xFD(-3)] — echoes byte-by-byte in a loop.
        let replicator: [u8; 3] = [ECHO, JMP_REL, 0xFD];

        let mut tape = vec![0u8; 128];
        tape[..3].copy_from_slice(&replicator);

        Echo::execute(&mut tape, 8192);

        // Default delay = 64. ECHO copies tape[rp] to tape[rp+64].
        // First half should be copied to second half.
        assert_eq!(&tape[64..67], &replicator);
        assert_eq!(&tape[67..128], &vec![0u8; 61]);
    }

    #[test]
    fn test_replicator_is_functional_fixed_point() {
        let replicator: [u8; 3] = [ECHO, JMP_REL, 0xFD];

        let mut tape1 = vec![0u8; 128];
        tape1[..3].copy_from_slice(&replicator);
        Echo::execute(&mut tape1, 8192);
        let copy1 = tape1[64..128].to_vec();

        let mut tape2 = vec![0u8; 128];
        tape2[..64].copy_from_slice(&copy1);
        Echo::execute(&mut tape2, 8192);
        let copy2 = tape2[64..128].to_vec();

        assert_eq!(copy1, copy2, "Echo replicator should be a fixed point");
    }

    #[test]
    fn test_write_pointer_always_constrained() {
        // No matter what we do, writes should always go to rp + delay
        let mut tape = make_tape(&[SET_DELAY, 10, INC, INC, INC, STORE, SKIP, STORE], 128);
        Echo::execute(&mut tape, 8192);
        // After SET_DELAY 10: delay=10, pc=2, rp=0
        // INC, INC, INC: acc=3, rp still 0
        // STORE: writes 3 to tape[rp(0)+delay(10)] = tape[10]
        assert_eq!(tape[10], 3);
        // SKIP: rp becomes 1
        // STORE: writes 3 to tape[rp(1)+delay(10)] = tape[11]
        assert_eq!(tape[11], 3);
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
            let steps = Echo::execute(&mut tape, 8192);
            prop_assert!(steps <= 8192);
        }

        #[test]
        fn random_programs_respect_step_limit(
            tape_data in prop::collection::vec(any::<u8>(), 1..256),
            limit in 1usize..1000
        ) {
            let mut tape = tape_data;
            let steps = Echo::execute(&mut tape, limit);
            prop_assert!(steps <= limit);
        }

        #[test]
        fn output_tape_same_length(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let original_len = tape_data.len();
            let mut tape = tape_data;
            Echo::execute(&mut tape, 8192);
            prop_assert_eq!(tape.len(), original_len);
        }
    }
}
