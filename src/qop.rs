use crate::substrate::Substrate;

/// The Qop (Queue-Operate-Produce) instruction set — a queue-based substrate.
///
/// Inspired by tag systems and pipeline architectures. Data flows one direction
/// through the tape via a read head (`head`) and write head (`tail`). There is
/// no random access by default — programs must process data in order.
///
/// State:
/// - `pc`: instruction pointer, starts at 0
/// - `head`: read pointer, starts at 0 (front of queue)
/// - `tail`: write pointer, starts at `tape.len() / 2` (back of queue)
/// - `acc`: accumulator register (u8), starts at 0
///
/// All pointer arithmetic wraps modulo tape length.
pub struct Qop;

// Instruction opcodes
const HALT: u8 = 0x00;
const PASS: u8 = 0x01;
const EAT: u8 = 0x02;
const SPIT: u8 = 0x03;
const SKIP: u8 = 0x04;
const GAP: u8 = 0x05;
const INC: u8 = 0x06;
const DEC: u8 = 0x07;
const XOR: u8 = 0x08;
const JMP_REL: u8 = 0x09;
const JZ: u8 = 0x0A;
const JNZ: u8 = 0x0B;
const SET_HEAD: u8 = 0x0C;
const SET_TAIL: u8 = 0x0D;
const GET_HEAD: u8 = 0x0E;
const GET_TAIL: u8 = 0x0F;

impl Substrate for Qop {
    fn execute(tape: &mut [u8], step_limit: usize) -> usize {
        let len = tape.len();
        if len == 0 {
            return 0;
        }

        let mut pc: usize = 0;
        let mut head: u8 = 0;
        let mut tail: u8 = (len / 2) as u8;
        let mut acc: u8 = 0;
        let mut steps: usize = 0;

        while pc < len && steps < step_limit {
            steps += 1;
            match tape[pc] {
                HALT => break,
                PASS => {
                    let src = head as usize % len;
                    let dst = tail as usize % len;
                    tape[dst] = tape[src];
                    head = head.wrapping_add(1);
                    tail = tail.wrapping_add(1);
                }
                EAT => {
                    acc = tape[head as usize % len];
                    head = head.wrapping_add(1);
                }
                SPIT => {
                    tape[tail as usize % len] = acc;
                    tail = tail.wrapping_add(1);
                }
                SKIP => {
                    head = head.wrapping_add(1);
                }
                GAP => {
                    tape[tail as usize % len] = 0;
                    tail = tail.wrapping_add(1);
                }
                INC => {
                    acc = acc.wrapping_add(1);
                }
                DEC => {
                    acc = acc.wrapping_sub(1);
                }
                XOR => {
                    acc ^= tape[head as usize % len];
                }
                JMP_REL => {
                    if pc + 1 >= len {
                        break;
                    }
                    let offset = tape[pc + 1] as i8;
                    let new_pc = pc as isize + 2 + offset as isize;
                    if new_pc < 0 {
                        break;
                    }
                    pc = new_pc as usize;
                    continue; // don't do pc += 1 below
                }
                JZ => {
                    if pc + 1 >= len {
                        break;
                    }
                    if acc == 0 {
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
                JNZ => {
                    if pc + 1 >= len {
                        break;
                    }
                    if acc != 0 {
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
                SET_HEAD => {
                    head = acc;
                }
                SET_TAIL => {
                    tail = acc;
                }
                GET_HEAD => {
                    acc = head;
                }
                GET_TAIL => {
                    acc = tail;
                }
                _ => {} // NOP (0x10-0xFF)
            }
            pc += 1;
        }

        steps
    }

    fn is_instruction(byte: u8) -> bool {
        byte <= GET_TAIL
    }

    fn disassemble(tape: &[u8]) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        let mut pc = 0;
        while pc < tape.len() {
            let b = tape[pc];
            let desc = match b {
                HALT => { let s = "HALT".to_string(); pc += 1; s }
                PASS => { let s = "PASS".to_string(); pc += 1; s }
                EAT => { let s = "EAT".to_string(); pc += 1; s }
                SPIT => { let s = "SPIT".to_string(); pc += 1; s }
                SKIP => { let s = "SKIP".to_string(); pc += 1; s }
                GAP => { let s = "GAP".to_string(); pc += 1; s }
                INC => { let s = "INC".to_string(); pc += 1; s }
                DEC => { let s = "DEC".to_string(); pc += 1; s }
                XOR => { let s = "XOR".to_string(); pc += 1; s }
                JMP_REL => {
                    if pc + 1 < tape.len() {
                        let offset = tape[pc + 1] as i8;
                        let target = pc as isize + 2 + offset as isize;
                        let s = format!("JMP_REL {offset:+} (-> {target})");
                        pc += 2;
                        s
                    } else {
                        let s = "JMP_REL ???".to_string();
                        pc += 1;
                        s
                    }
                }
                JZ => {
                    if pc + 1 < tape.len() {
                        let offset = tape[pc + 1] as i8;
                        let target = pc as isize + 2 + offset as isize;
                        let s = format!("JZ {offset:+} (-> {target})");
                        pc += 2;
                        s
                    } else {
                        let s = "JZ ???".to_string();
                        pc += 1;
                        s
                    }
                }
                JNZ => {
                    if pc + 1 < tape.len() {
                        let offset = tape[pc + 1] as i8;
                        let target = pc as isize + 2 + offset as isize;
                        let s = format!("JNZ {offset:+} (-> {target})");
                        pc += 2;
                        s
                    } else {
                        let s = "JNZ ???".to_string();
                        pc += 1;
                        s
                    }
                }
                SET_HEAD => { let s = "SET_HEAD".to_string(); pc += 1; s }
                SET_TAIL => { let s = "SET_TAIL".to_string(); pc += 1; s }
                GET_HEAD => { let s = "GET_HEAD".to_string(); pc += 1; s }
                GET_TAIL => { let s = "GET_TAIL".to_string(); pc += 1; s }
                _ => { let s = "NOP".to_string(); pc += 1; s }
            };
            let _ = writeln!(out, "{:04X}: {:02X}  {desc}", pc - if matches!(b, JMP_REL | JZ | JNZ) && pc >= 2 { 2 } else { 1 }, b);
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
        let mut tape = make_tape(&[HALT, INC, INC], 128);
        let steps = Qop::execute(&mut tape, 8192);
        assert_eq!(steps, 1);
    }

    #[test]
    fn test_pass_copies_one_byte() {
        // head=0, tail=64. PASS copies tape[0] to tape[64].
        let mut tape = make_tape(&[PASS], 128);
        Qop::execute(&mut tape, 8192);
        assert_eq!(tape[64], PASS); // copied the PASS instruction itself
    }

    #[test]
    fn test_eat_loads_acc() {
        // EAT reads tape[head=0] into acc. tape[0] = EAT = 0x02.
        // Then SPIT writes acc to tape[tail=64].
        let mut tape = make_tape(&[EAT, SPIT], 128);
        Qop::execute(&mut tape, 8192);
        assert_eq!(tape[64], EAT); // acc got the EAT byte, spit it to tail
    }

    #[test]
    fn test_spit_writes_acc() {
        // INC acc to 42, then SPIT to tape[64].
        let mut program = vec![INC; 42];
        program.push(SPIT);
        let mut tape = make_tape(&program, 128);
        Qop::execute(&mut tape, 8192);
        assert_eq!(tape[64], 42);
    }

    #[test]
    fn test_skip_advances_head() {
        // SKIP advances head 3 times, GET_HEAD reads head into acc, SPIT writes to tape[64].
        let mut tape = make_tape(&[SKIP, SKIP, SKIP, GET_HEAD, SPIT], 128);
        Qop::execute(&mut tape, 8192);
        assert_eq!(tape[64], 3);
    }

    #[test]
    fn test_gap_writes_zero() {
        // Set tape[64] to 0xFF. GAP writes 0 to tape[64] and advances tail.
        let mut tape = make_tape(&[GAP], 128);
        tape[64] = 0xFF;
        Qop::execute(&mut tape, 8192);
        assert_eq!(tape[64], 0);
    }

    #[test]
    fn test_inc_dec() {
        // INC 3 times, DEC once -> acc = 2. SPIT to tape[64].
        let mut tape = make_tape(&[INC, INC, INC, DEC, SPIT], 128);
        Qop::execute(&mut tape, 8192);
        assert_eq!(tape[64], 2);
    }

    #[test]
    fn test_inc_wraps() {
        // DEC from 0 gives 255, then INC wraps back to 0.
        let mut tape = make_tape(&[DEC, INC, SPIT], 128);
        Qop::execute(&mut tape, 8192);
        assert_eq!(tape[64], 0);
    }

    #[test]
    fn test_dec_wraps() {
        // DEC from 0 wraps to 255.
        let mut tape = make_tape(&[DEC, SPIT], 128);
        Qop::execute(&mut tape, 8192);
        assert_eq!(tape[64], 255);
    }

    #[test]
    fn test_xor() {
        // Set tape[head=0] to something, XOR with acc.
        // acc starts at 0. tape[0] = XOR = 0x08. XOR: acc ^= tape[0] = 0x08.
        let mut tape = make_tape(&[XOR, SPIT], 128);
        Qop::execute(&mut tape, 8192);
        assert_eq!(tape[64], XOR); // 0 ^ 0x08 = 0x08
    }

    #[test]
    fn test_jmp_rel_forward() {
        // JMP_REL +1 skips one byte, landing at pc=3.
        // [JMP_REL, 0x01, INC, SPIT, ...] -> skips INC, acc stays 0.
        let mut tape = make_tape(&[JMP_REL, 0x01, INC, SPIT], 128);
        Qop::execute(&mut tape, 8192);
        assert_eq!(tape[64], 0); // INC was skipped
    }

    #[test]
    fn test_jmp_rel_backward() {
        // NOP, INC, JMP_REL, -4 -> jumps from pc=2 to pc=2+2+(-4)=0.
        // This creates a loop: NOP, INC, jump back to NOP.
        let mut tape = make_tape(&[0xFF, INC, JMP_REL, 0xFC], 128);
        let steps = Qop::execute(&mut tape, 100);
        assert_eq!(steps, 100); // infinite loop hits step limit
    }

    #[test]
    fn test_jmp_rel_negative_target_terminates() {
        // JMP_REL with offset that goes negative should terminate.
        let mut tape = make_tape(&[JMP_REL, 0x80], 128); // offset = -128, target = 0+2-128 = -126
        let steps = Qop::execute(&mut tape, 8192);
        assert_eq!(steps, 1);
    }

    #[test]
    fn test_jz_taken() {
        // acc=0, JZ should jump.
        // JZ +1 skips one byte.
        let mut tape = make_tape(&[JZ, 0x01, INC, SPIT], 128);
        Qop::execute(&mut tape, 8192);
        assert_eq!(tape[64], 0); // INC was skipped
    }

    #[test]
    fn test_jz_not_taken() {
        // acc=1, JZ should not jump.
        let mut tape = make_tape(&[INC, JZ, 0x01, INC, SPIT], 128);
        Qop::execute(&mut tape, 8192);
        assert_eq!(tape[64], 2); // both INCs executed
    }

    #[test]
    fn test_jnz_taken() {
        // acc=1, JNZ should jump.
        let mut tape = make_tape(&[INC, JNZ, 0x01, INC, SPIT], 128);
        Qop::execute(&mut tape, 8192);
        assert_eq!(tape[64], 1); // second INC was skipped
    }

    #[test]
    fn test_jnz_not_taken() {
        // acc=0, JNZ should not jump.
        let mut tape = make_tape(&[JNZ, 0x01, INC, SPIT], 128);
        Qop::execute(&mut tape, 8192);
        assert_eq!(tape[64], 1); // INC executed
    }

    #[test]
    fn test_set_head() {
        // Set acc to 10 via INC, then SET_HEAD. EAT should read tape[10].
        let mut tape = make_tape(&[INC; 10], 128);
        tape[10] = SET_HEAD;
        tape[11] = EAT;
        tape[12] = SPIT;
        tape[42] = 0x77; // tape[10] after SET_HEAD, head=10. EAT reads tape[10]=SET_HEAD=0x0C.
        Qop::execute(&mut tape, 8192);
        // acc = 10 after 10 INCs. SET_HEAD sets head=10. EAT reads tape[10] = 0x0C (SET_HEAD).
        assert_eq!(tape[64], SET_HEAD);
    }

    #[test]
    fn test_set_tail() {
        // Set acc to 100 via many INCs, then SET_TAIL. SPIT should write to tape[100].
        let mut program: Vec<u8> = vec![INC; 100];
        program.push(SET_TAIL);
        program.push(SPIT);
        let mut tape = make_tape(&program, 128);
        Qop::execute(&mut tape, 8192);
        assert_eq!(tape[100], 100); // acc=100, SET_TAIL makes tail=100, SPIT writes 100 there
    }

    #[test]
    fn test_get_head() {
        // SKIP 3 times (head=3), GET_HEAD (acc=3), SPIT to tape[64].
        let mut tape = make_tape(&[SKIP, SKIP, SKIP, GET_HEAD, SPIT], 128);
        Qop::execute(&mut tape, 8192);
        assert_eq!(tape[64], 3);
    }

    #[test]
    fn test_get_tail() {
        // GET_TAIL (acc=64), SPIT (writes 64 to tape[64], tail becomes 65).
        let mut tape = make_tape(&[GET_TAIL, SPIT], 128);
        Qop::execute(&mut tape, 8192);
        assert_eq!(tape[64], 64);
    }

    #[test]
    fn test_nop_bytes() {
        // Bytes 0x10-0xFF are NOPs. Should just advance pc.
        let mut tape = make_tape(&[0x10, 0x80, 0xFF, INC, SPIT], 128);
        Qop::execute(&mut tape, 8192);
        assert_eq!(tape[64], 1); // only one INC executed
    }

    #[test]
    fn test_empty_tape() {
        let mut tape: Vec<u8> = vec![];
        let steps = Qop::execute(&mut tape, 8192);
        assert_eq!(steps, 0);
    }

    #[test]
    fn test_step_limit() {
        // Infinite loop: JMP_REL -2 (jumps to itself).
        let mut tape = make_tape(&[JMP_REL, 0xFE], 128);
        let steps = Qop::execute(&mut tape, 100);
        assert_eq!(steps, 100);
    }

    // --- Replicator tests ---

    #[test]
    fn test_3_byte_self_replicator() {
        // [PASS, JMP_REL, 0xFD(-3)] — copies byte-by-byte in a loop.
        let replicator: [u8; 3] = [PASS, JMP_REL, 0xFD];

        let mut tape = vec![0u8; 128];
        tape[..3].copy_from_slice(&replicator);

        Qop::execute(&mut tape, 8192);

        // First half should be copied to second half.
        assert_eq!(&tape[64..67], &replicator, "replicator bytes should be copied");
        // Bytes 3..64 were 0 in the original, should be 0 in the copy.
        assert_eq!(&tape[67..128], &vec![0u8; 61], "padding should be copied as zeros");
    }

    #[test]
    fn test_replicator_is_functional_fixed_point() {
        let replicator: [u8; 3] = [PASS, JMP_REL, 0xFD];

        // Run 1: original replicates.
        let mut tape1 = vec![0u8; 128];
        tape1[..3].copy_from_slice(&replicator);
        Qop::execute(&mut tape1, 8192);
        let copy1 = tape1[64..128].to_vec();

        // Run 2: the copy should also replicate.
        let mut tape2 = vec![0u8; 128];
        tape2[..64].copy_from_slice(&copy1);
        Qop::execute(&mut tape2, 8192);
        let copy2 = tape2[64..128].to_vec();

        assert_eq!(copy1, copy2, "Qop replicator should be a fixed point");
    }

    #[test]
    fn test_multiple_pass_replicator() {
        // A program of 64 PASS bytes should also self-replicate (no loop needed).
        let mut tape = vec![PASS; 64];
        tape.extend(vec![0u8; 64]);
        assert_eq!(tape.len(), 128);

        Qop::execute(&mut tape, 8192);

        assert_eq!(&tape[64..128], &vec![PASS; 64], "64 PASS bytes should copy themselves");
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
            let steps = Qop::execute(&mut tape, 8192);
            prop_assert!(steps <= 8192);
        }

        #[test]
        fn random_programs_respect_step_limit(
            tape_data in prop::collection::vec(any::<u8>(), 1..256),
            limit in 1usize..1000
        ) {
            let mut tape = tape_data;
            let steps = Qop::execute(&mut tape, limit);
            prop_assert!(steps <= limit);
        }

        #[test]
        fn output_tape_same_length(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let original_len = tape_data.len();
            let mut tape = tape_data;
            Qop::execute(&mut tape, 8192);
            prop_assert_eq!(tape.len(), original_len);
        }
    }
}
