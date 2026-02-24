use crate::substrate::Substrate;

/// The SUBLEQ instruction set from Section 3.2 of the paper.
///
/// SUBLEQ is one of the simplest Turing-complete languages, possessing a
/// single instruction. The only state is the program counter (pc).
///
/// Each instruction reads three values a, b, c from consecutive tape positions
/// starting at pc. The instruction executes (in C-like syntax):
///
///   *a -= *b; if (*a <= 0) { goto c; } else { goto pc + 3; }
///
/// All addresses wrap modulo the tape length. The `<= 0` comparison interprets
/// the byte as signed (i8). The program terminates when pc would require
/// reading past the end of the tape.
///
/// This is a counterexample substrate from the paper: self-replication is
/// possible (the smallest hand-crafted self-replicator is 60 bytes), but it
/// does not spontaneously emerge from random initialization.
pub struct Subleq;

struct SubleqBattleState {
    pc: usize,
}

fn subleq_battle_init(_tape_len: usize, start_pc: usize) -> SubleqBattleState {
    SubleqBattleState { pc: start_pc }
}

fn subleq_battle_step(state: &mut SubleqBattleState, tape: &mut [u8]) -> bool {
    let len = tape.len();
    if state.pc + 2 >= len {
        return false;
    }

    let a = tape[state.pc] as usize % len;
    let b = tape[state.pc + 1] as usize % len;

    tape[a] = tape[a].wrapping_sub(tape[b]);

    if (tape[a] as i8) <= 0 {
        state.pc = tape[state.pc + 2] as usize;
    } else {
        state.pc += 3;
    }

    true
}

impl Substrate for Subleq {
    fn execute(tape: &mut [u8], step_limit: usize) -> usize {
        let len = tape.len();
        if len < 3 {
            return 0;
        }

        let mut pc: usize = 0;
        let mut steps: usize = 0;

        while pc + 2 < len && steps < step_limit {
            steps += 1;

            let a = tape[pc] as usize % len;
            let b = tape[pc + 1] as usize % len;

            tape[a] = tape[a].wrapping_sub(tape[b]);

            if (tape[a] as i8) <= 0 {
                // Branch target is read AFTER the subtraction, matching the
                // reference implementation. This matters when a == pc + 2.
                pc = tape[pc + 2] as usize;
            } else {
                pc += 3;
            }
        }

        steps
    }

    fn execute_battle(tape: &mut [u8], ps: usize, step_limit: usize) -> usize {
        let len = tape.len();
        if len < 3 {
            return 0;
        }
        let mut a = subleq_battle_init(len, 0);
        let mut b = subleq_battle_init(len, ps);
        let mut steps = 0;
        let mut halted_a = false;
        let mut halted_b = false;
        while steps < step_limit && (!halted_a || !halted_b) {
            if !halted_a {
                halted_a = !subleq_battle_step(&mut a, tape);
                steps += 1;
                if steps >= step_limit {
                    break;
                }
            }
            if !halted_b {
                halted_b = !subleq_battle_step(&mut b, tape);
                steps += 1;
            }
        }
        steps
    }

    fn is_instruction(_byte: u8) -> bool {
        // In SUBLEQ, every byte is part of an instruction triplet (address or
        // branch target). There are no no-op bytes.
        true
    }

    fn disassemble(tape: &[u8]) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        let mut pc = 0;
        while pc + 2 < tape.len() {
            let a = tape[pc];
            let b = tape[pc + 1];
            let c = tape[pc + 2];
            let _ = writeln!(
                out,
                "{pc:04X}: [{a:02X} {b:02X} {c:02X}]  *{a} -= *{b}; if <=0 goto {c}"
            );
            pc += 3;
        }
        // Trailing bytes that don't form a complete triplet.
        for (i, &byte) in tape.iter().enumerate().skip(pc) {
            let _ = writeln!(out, "{i:04X}: {byte:02X}     (trailing)");
        }
        out
    }
}

/// The RSUBLEQ4 variant from Section 3.2 of the paper.
///
/// A relative-addressing variant of SUBLEQ that reads 4 values per instruction.
/// Each instruction reads a, b, c, d from consecutive tape positions starting
/// at pc. The instruction executes:
///
///   *(pc+a) = *(pc+b) - *(pc+c); if (*(pc+a) <= 0) { goto pc+d; } else { goto pc+4; }
///
/// Data address offsets a, b, c are **unsigned** (u8) when added to pc, then
/// wrapped modulo tape length. The branch offset d is **signed** (i8).
/// The program terminates when pc would require reading past the end of
/// the tape, or when the branch target is negative.
///
/// This variant admits a significantly shorter self-replicator (25 bytes)
/// than standard SUBLEQ (60 bytes).
pub struct Rsubleq4;

struct Rsubleq4BattleState {
    pc: usize,
}

fn rsubleq4_battle_init(_tape_len: usize, start_pc: usize) -> Rsubleq4BattleState {
    Rsubleq4BattleState { pc: start_pc }
}

fn rsubleq4_battle_step(state: &mut Rsubleq4BattleState, tape: &mut [u8]) -> bool {
    let len = tape.len();
    if state.pc + 3 >= len {
        return false;
    }

    // Data offsets are unsigned.
    let a = tape[state.pc] as usize;
    let b = tape[state.pc + 1] as usize;
    let c = tape[state.pc + 2] as usize;

    let addr_a = (state.pc + a) % len;
    let addr_b = (state.pc + b) % len;
    let addr_c = (state.pc + c) % len;

    tape[addr_a] = tape[addr_b].wrapping_sub(tape[addr_c]);

    if (tape[addr_a] as i8) <= 0 {
        let d = tape[state.pc + 3] as i8;
        let new_pc = state.pc as isize + d as isize;
        if new_pc < 0 {
            return false;
        }
        state.pc = new_pc as usize;
    } else {
        state.pc += 4;
    }

    true
}

impl Substrate for Rsubleq4 {
    fn disassemble(tape: &[u8]) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        let mut pc = 0;
        while pc + 3 < tape.len() {
            let a = tape[pc];
            let b = tape[pc + 1];
            let c = tape[pc + 2];
            let d = tape[pc + 3] as i8;
            let _ = writeln!(
                out,
                "{pc:04X}: [{a:02X} {b:02X} {c:02X} {:02X}]  *(pc+{a}) = *(pc+{b}) - *(pc+{c}); if <=0 goto pc{d:+}",
                tape[pc + 3]
            );
            pc += 4;
        }
        for (i, &byte) in tape.iter().enumerate().skip(pc) {
            let _ = writeln!(out, "{i:04X}: {byte:02X}     (trailing)");
        }
        out
    }

    fn execute(tape: &mut [u8], step_limit: usize) -> usize {
        let len = tape.len();
        if len < 4 {
            return 0;
        }

        let mut pc: usize = 0;
        let mut steps: usize = 0;

        while pc + 3 < len && steps < step_limit {
            steps += 1;

            // Data offsets are unsigned.
            let a = tape[pc] as usize;
            let b = tape[pc + 1] as usize;
            let c = tape[pc + 2] as usize;

            let addr_a = (pc + a) % len;
            let addr_b = (pc + b) % len;
            let addr_c = (pc + c) % len;

            tape[addr_a] = tape[addr_b].wrapping_sub(tape[addr_c]);

            if (tape[addr_a] as i8) <= 0 {
                // Branch offset is read AFTER the subtraction, matching the
                // reference implementation. This matters when addr_a == pc + 3.
                let d = tape[pc + 3] as i8;
                let new_pc = pc as isize + d as isize;
                if new_pc < 0 {
                    break;
                }
                pc = new_pc as usize;
            } else {
                pc += 4;
            }
        }

        steps
    }

    fn execute_battle(tape: &mut [u8], ps: usize, step_limit: usize) -> usize {
        let len = tape.len();
        if len < 4 {
            return 0;
        }
        let mut a = rsubleq4_battle_init(len, 0);
        let mut b = rsubleq4_battle_init(len, ps);
        let mut steps = 0;
        let mut halted_a = false;
        let mut halted_b = false;
        while steps < step_limit && (!halted_a || !halted_b) {
            if !halted_a {
                halted_a = !rsubleq4_battle_step(&mut a, tape);
                steps += 1;
                if steps >= step_limit {
                    break;
                }
            }
            if !halted_b {
                halted_b = !rsubleq4_battle_step(&mut b, tape);
                steps += 1;
            }
        }
        steps
    }

    fn is_instruction(_byte: u8) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- SUBLEQ tests ---

    #[test]
    fn test_subleq_basic_subtraction() {
        // tape: a=3, b=4, c=99 (oob branch target)
        // tape[3]=10, tape[4]=3
        // tape[3] -= tape[4] => 10 - 3 = 7. 7 > 0 => pc += 3.
        // pc=3: a=tape[3]=7, b=tape[4]=3, c=tape[5]=0.
        // tape[7] -= tape[3] => 0 - 7 = 249. 249 as i8 = -7 <= 0 => goto 0.
        // Verify the first subtraction happened correctly.
        let mut tape = vec![3, 4, 99, 10, 3, 0, 0, 0];
        Subleq::execute(&mut tape, 1);
        assert_eq!(tape[3], 7); // 10 - 3
    }

    #[test]
    fn test_subleq_branch_on_zero() {
        // Set up so *a becomes exactly 0.
        // tape[3]=5, tape[4]=5. a=3, b=4. tape[3] -= tape[4] => 5-5=0. 0<=0 => goto c.
        let mut tape = vec![3, 4, 7, 5, 5, 0, 0, 0];
        // c=7 which is tape[2]=7. goto 7. pc=7, but pc+2=9 >= 8 => terminate.
        let steps = Subleq::execute(&mut tape, 100);
        assert_eq!(steps, 1);
        assert_eq!(tape[3], 0);
    }

    #[test]
    fn test_subleq_branch_on_negative() {
        // tape[3]=2, tape[4]=5. a=3, b=4. tape[3] -= tape[4] => 2-5=253 (wrapping).
        // 253 as i8 = -3 <= 0 => goto c.
        let mut tape = vec![3, 4, 7, 2, 5, 0, 0, 0];
        let steps = Subleq::execute(&mut tape, 100);
        assert_eq!(steps, 1);
        assert_eq!(tape[3], 253); // 2 - 5 wrapping
    }

    #[test]
    fn test_subleq_no_branch_positive() {
        // tape[3]=10, tape[4]=3. a=3, b=4. tape[3] -= tape[4] => 10-3=7. 7>0 => pc+=3.
        // pc=3. a=tape[3]=7, b=tape[4]=3, c=tape[5]=0.
        // tape[7] -= tape[3] => 0-7=249. 249 as i8 = -7 <= 0 => goto 0. Loops.
        let mut tape = vec![3, 4, 0, 10, 3, 0, 0, 0];
        let steps = Subleq::execute(&mut tape, 3);
        assert_eq!(steps, 3);
    }

    #[test]
    fn test_subleq_terminates_oob() {
        // c=255 (out of bounds for an 8-byte tape). After branching, pc=255.
        // pc+2=257 >= 8, so terminates.
        let mut tape = vec![3, 4, 255, 5, 5, 0, 0, 0];
        let steps = Subleq::execute(&mut tape, 100);
        assert_eq!(steps, 1);
    }

    #[test]
    fn test_subleq_empty_tape() {
        let mut tape: Vec<u8> = vec![];
        let steps = Subleq::execute(&mut tape, 100);
        assert_eq!(steps, 0);
    }

    #[test]
    fn test_subleq_small_tape() {
        let mut tape = vec![0, 0];
        let steps = Subleq::execute(&mut tape, 100);
        assert_eq!(steps, 0);
    }

    #[test]
    fn test_subleq_step_limit() {
        // Self-subtracting loop: a=b=0, c=0. tape[0] -= tape[0] => always 0.
        // 0 <= 0 => goto 0. Infinite loop.
        let mut tape = vec![0, 0, 0, 0, 0, 0, 0, 0];
        let steps = Subleq::execute(&mut tape, 50);
        assert_eq!(steps, 50);
    }

    #[test]
    fn test_subleq_address_wraps() {
        // a=200, b=201, on a 64-byte tape. 200%64=8, 201%64=9.
        let mut tape = vec![0u8; 64];
        tape[0] = 200; // a = 200 % 64 = 8
        tape[1] = 201; // b = 201 % 64 = 9
        tape[2] = 63; // c = 63 (goto 63, which is oob for next instruction)
        tape[8] = 10;
        tape[9] = 3;
        Subleq::execute(&mut tape, 1);
        assert_eq!(tape[8], 7); // 10 - 3 = 7
    }

    #[test]
    fn test_subleq_self_modifying() {
        // a=0, b=1, c=0. tape[0] -= tape[1].
        // tape[0]=0, tape[1]=1. tape[0] = 0-1 = 255. 255 as i8 = -1 <= 0 => goto tape[2]=0.
        // Now tape[0]=255 (a=255%8=7), tape[1]=1 (b=1). tape[7] -= tape[1] => 0-1=255.
        // Self-modifying: the instruction operands change because we wrote to tape[0].
        let mut tape = vec![0, 1, 0, 0, 0, 0, 0, 0];
        Subleq::execute(&mut tape, 2);
        assert_eq!(tape[0], 255); // first instruction modified tape[0]
        assert_eq!(tape[7], 255); // second instruction used modified a=255%8=7
    }

    // --- RSUBLEQ4 tests ---

    #[test]
    fn test_rsubleq4_basic() {
        // pc=0. a=4, b=5, c=6 (unsigned offsets), d=7 (signed, but positive).
        // addr_a = (0+4)%8=4, addr_b = (0+5)%8=5, addr_c = (0+6)%8=6.
        // tape[4] = tape[5] - tape[6] = 10 - 3 = 7. 7 > 0 => pc += 4.
        // pc=4: a=tape[4]=7, b=tape[5]=10, c=tape[6]=3, d=tape[7]=0.
        // addr_a = (4+7)%8=3, addr_b = (4+10)%8=6, addr_c = (4+3)%8=7.
        // tape[3] = tape[6] - tape[7] = 3 - 0 = 3. 3 > 0 => pc += 4 = 8.
        // pc=8, pc+3=11 >= 8 => terminate.
        let mut tape = vec![4, 5, 6, 7, 0, 10, 3, 0];
        let steps = Rsubleq4::execute(&mut tape, 100);
        assert_eq!(steps, 2);
        assert_eq!(tape[4], 7); // 10 - 3
    }

    #[test]
    fn test_rsubleq4_large_unsigned_offset() {
        // a=0xFF (255 unsigned). pc=0. addr_a = (0+255)%8 = 7.
        // b=5, c=6. addr_b = (0+5)%8=5, addr_c = (0+6)%8=6.
        // tape[7] = tape[5] - tape[6] = 10 - 3 = 7. 7 > 0 => pc += 4.
        let mut tape: Vec<u8> = vec![0xFF, 5, 6, 4, 0, 10, 3, 0];
        Rsubleq4::execute(&mut tape, 1);
        assert_eq!(tape[7], 7); // 10 - 3
    }

    #[test]
    fn test_rsubleq4_branch_taken() {
        // tape[4] = tape[5] - tape[6]. If result <= 0, goto pc + d.
        // Set tape[5]=3, tape[6]=5. Result = 3-5 = 254 (wrapping). 254 as i8 = -2 <= 0.
        // d = tape[3] = 0xFC (-4 as i8). pc goes to 0 + (-4) = -4, terminates.
        let mut tape: Vec<u8> = vec![4, 5, 6, 0xFC, 0, 3, 5, 0];
        let steps = Rsubleq4::execute(&mut tape, 100);
        assert_eq!(steps, 1); // terminates because new_pc = -4 < 0
        assert_eq!(tape[4], 254); // 3 - 5 wrapping
    }

    #[test]
    fn test_rsubleq4_branch_forward() {
        // d=8 means skip to pc+8, which is out of bounds for an 8-byte tape.
        // tape[5]=0, tape[6]=0. Result = 0-0 = 0. 0 <= 0 => goto pc+d.
        // d = tape[3] = 8. pc = 0+8 = 8. pc+3 = 11 >= 8 => terminate.
        let mut tape: Vec<u8> = vec![4, 5, 6, 8, 0, 0, 0, 0];
        let steps = Rsubleq4::execute(&mut tape, 100);
        assert_eq!(steps, 1);
    }

    #[test]
    fn test_rsubleq4_empty_tape() {
        let mut tape: Vec<u8> = vec![];
        let steps = Rsubleq4::execute(&mut tape, 100);
        assert_eq!(steps, 0);
    }

    #[test]
    fn test_rsubleq4_small_tape() {
        let mut tape = vec![0, 0, 0];
        let steps = Rsubleq4::execute(&mut tape, 100);
        assert_eq!(steps, 0);
    }

    #[test]
    fn test_rsubleq4_step_limit() {
        // Self-loop: a=0, b=0, c=0, d=0. tape[pc+0] = tape[pc+0] - tape[pc+0] = 0.
        // 0 <= 0 => goto pc+0 = 0. Infinite loop.
        let mut tape = vec![0, 0, 0, 0, 0, 0, 0, 0];
        let steps = Rsubleq4::execute(&mut tape, 50);
        assert_eq!(steps, 50);
    }

    #[test]
    fn test_subleq_is_instruction() {
        assert!(Subleq::is_instruction(0));
        assert!(Subleq::is_instruction(42));
        assert!(Subleq::is_instruction(255));
    }

    #[test]
    fn test_rsubleq4_is_instruction() {
        assert!(Rsubleq4::is_instruction(0));
        assert!(Rsubleq4::is_instruction(42));
        assert!(Rsubleq4::is_instruction(255));
    }

    /// Verify the 25-byte RSUBLEQ4 self-replicator from Section 3.2 of the paper.
    ///
    /// The replicator uses positions 8-9 as loop counter working memory, so
    /// the copy is not byte-identical to the original at those positions.
    /// We verify functional self-replication: the copy should also produce
    /// a copy of itself when run with food.
    #[test]
    fn test_rsubleq4_paper_self_replicator() {
        // From the paper: 9 16 20 4 4 5 19 4 0 0 12 4 -3 -3 9 4 -8 8 -7 -12 0 -1 -1 -64 -73
        // As u8 (two's complement):
        let replicator: [u8; 25] = [
            9, 16, 20, 4, 4, 5, 19, 4, 0, 0, 12, 4, 253, 253, 9, 4, // -3, -3, 9, 4
            248, 8, 249, 244, // -8, 8, -7, -12
            0, 255, 255, 192, 183, // 0, -1, -1, -64, -73
        ];

        // Run 1: place replicator in first 64 bytes, zeros as food.
        let mut tape = vec![0u8; 128];
        tape[..25].copy_from_slice(&replicator);

        Rsubleq4::execute(&mut tape, 8192);

        // The copy is in the second half. It may differ at working-memory
        // positions (8, 9) from the original, but it should be functionally
        // identical.
        let copy = tape[64..128].to_vec();

        // Verify the non-counter bytes match exactly.
        assert_eq!(&copy[..8], &replicator[..8], "bytes 0-7 should match");
        assert_eq!(
            &copy[10..25],
            &replicator[10..25],
            "bytes 10-24 should match"
        );

        // Run 2: the copy should also self-replicate.
        let mut tape2 = vec![0u8; 128];
        tape2[..64].copy_from_slice(&copy);

        Rsubleq4::execute(&mut tape2, 8192);

        let copy2 = tape2[64..128].to_vec();

        // The copy-of-copy should match the copy (stable fixed point).
        assert_eq!(
            &copy2[..25],
            &copy[..25],
            "RSUBLEQ4 copy should also self-replicate (fixed point)"
        );
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn subleq_never_panics(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let mut tape = tape_data;
            let steps = Subleq::execute(&mut tape, 8192);
            prop_assert!(steps <= 8192);
        }

        #[test]
        fn subleq_respects_step_limit(
            tape_data in prop::collection::vec(any::<u8>(), 1..256),
            limit in 1usize..1000
        ) {
            let mut tape = tape_data;
            let steps = Subleq::execute(&mut tape, limit);
            prop_assert!(steps <= limit);
        }

        #[test]
        fn subleq_preserves_tape_length(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let original_len = tape_data.len();
            let mut tape = tape_data;
            Subleq::execute(&mut tape, 8192);
            prop_assert_eq!(tape.len(), original_len);
        }

        #[test]
        fn rsubleq4_never_panics(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let mut tape = tape_data;
            let steps = Rsubleq4::execute(&mut tape, 8192);
            prop_assert!(steps <= 8192);
        }

        #[test]
        fn rsubleq4_respects_step_limit(
            tape_data in prop::collection::vec(any::<u8>(), 1..256),
            limit in 1usize..1000
        ) {
            let mut tape = tape_data;
            let steps = Rsubleq4::execute(&mut tape, limit);
            prop_assert!(steps <= limit);
        }

        #[test]
        fn rsubleq4_preserves_tape_length(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let original_len = tape_data.len();
            let mut tape = tape_data;
            Rsubleq4::execute(&mut tape, 8192);
            prop_assert_eq!(tape.len(), original_len);
        }
    }
}
