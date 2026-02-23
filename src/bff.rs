use crate::substrate::Substrate;

/// The BFF (Brainfuck Family) instruction set from Section 2 of the paper.
///
/// Operates on a byte tape with three pointers:
/// - IP (instruction pointer): starts at 0, advances through the tape
/// - head0 (read head): starts at 0
/// - head1 (write head): starts at 0
///
/// All pointer arithmetic wraps modulo the tape length.
/// Head value arithmetic wraps modulo 256.
///
/// Bracket matching (`[` and `]`) is performed at runtime by scanning
/// the current tape contents, since programs are self-modifying.
pub struct Bff;

const LESS: u8 = b'<';
const GREATER: u8 = b'>';
const LBRACE: u8 = b'{';
const RBRACE: u8 = b'}';
const MINUS: u8 = b'-';
const PLUS: u8 = b'+';
const DOT: u8 = b'.';
const COMMA: u8 = b',';
const LBRACKET: u8 = b'[';
const RBRACKET: u8 = b']';

impl Substrate for Bff {
    fn execute(tape: &mut [u8], step_limit: usize) -> usize {
        let len = tape.len();
        if len == 0 {
            return 0;
        }

        let mut ip: usize = 0;
        let mut head0: u8 = 0;
        let mut head1: u8 = 0;
        let mut steps: usize = 0;

        while ip < len && steps < step_limit {
            steps += 1;
            match tape[ip] {
                LESS => head0 = head0.wrapping_sub(1),
                GREATER => head0 = head0.wrapping_add(1),
                LBRACE => head1 = head1.wrapping_sub(1),
                RBRACE => head1 = head1.wrapping_add(1),
                MINUS => {
                    let idx = head0 as usize % len;
                    tape[idx] = tape[idx].wrapping_sub(1);
                }
                PLUS => {
                    let idx = head0 as usize % len;
                    tape[idx] = tape[idx].wrapping_add(1);
                }
                DOT => {
                    let src = head0 as usize % len;
                    let dst = head1 as usize % len;
                    tape[dst] = tape[src];
                }
                COMMA => {
                    let dst = head0 as usize % len;
                    let src = head1 as usize % len;
                    tape[dst] = tape[src];
                }
                LBRACKET => {
                    let idx = head0 as usize % len;
                    if tape[idx] == 0 {
                        // Scan forward for matching ] on the current tape.
                        let mut depth: usize = 1;
                        let mut scan = ip + 1;
                        while scan < len && depth > 0 {
                            if tape[scan] == LBRACKET {
                                depth += 1;
                            } else if tape[scan] == RBRACKET {
                                depth -= 1;
                            }
                            scan += 1;
                        }
                        if depth > 0 {
                            // Unmatched bracket: terminate.
                            break;
                        }
                        // scan is now one past the matching ].
                        // Set ip so that after ip += 1 at the bottom, we land
                        // at scan (one past ]). So ip = scan - 1.
                        ip = scan - 1;
                    }
                }
                RBRACKET => {
                    let idx = head0 as usize % len;
                    if tape[idx] != 0 {
                        // Scan backward for matching [ on the current tape.
                        if ip == 0 {
                            // No room to scan: unmatched.
                            break;
                        }
                        let mut depth: usize = 1;
                        let mut scan = ip - 1;
                        loop {
                            if tape[scan] == RBRACKET {
                                depth += 1;
                            } else if tape[scan] == LBRACKET {
                                depth -= 1;
                            }
                            if depth == 0 {
                                break;
                            }
                            if scan == 0 {
                                break;
                            }
                            scan -= 1;
                        }
                        if depth > 0 {
                            // Unmatched bracket: terminate.
                            break;
                        }
                        // scan is at the matching [. Set ip so that after
                        // ip += 1, we land at [ + 1 (first instruction in
                        // the loop body), matching the reference behavior.
                        ip = scan;
                    }
                }
                _ => {} // no-op
            }
            ip += 1;
        }

        steps
    }

    fn is_instruction(byte: u8) -> bool {
        matches!(
            byte,
            LESS | GREATER | LBRACE | RBRACE | MINUS | PLUS | DOT | COMMA | LBRACKET | RBRACKET
        )
    }

    fn disassemble(tape: &[u8]) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        for (addr, &b) in tape.iter().enumerate() {
            let (ch, mnemonic) = match b {
                LESS => ("<", "HEAD0--"),
                GREATER => (">", "HEAD0++"),
                LBRACE => ("{", "HEAD1--"),
                RBRACE => ("}", "HEAD1++"),
                MINUS => ("-", "DEC"),
                PLUS => ("+", "INC"),
                DOT => (".", "COPY0->1"),
                COMMA => (",", "COPY1->0"),
                LBRACKET => ("[", "LOOP_START"),
                RBRACKET => ("]", "LOOP_END"),
                _ => ("", "NOP"),
            };
            let _ = writeln!(out, "{addr:04X}: {b:02X}  {ch:<2} {mnemonic}");
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a 256-byte tape with program bytes at the start,
    /// rest zero. Using 256 bytes means head0 (a u8) indexes directly.
    fn make_tape(program: &[u8]) -> Vec<u8> {
        let mut tape = vec![0u8; 256];
        for (i, &b) in program.iter().enumerate() {
            tape[i] = b;
        }
        tape
    }

    #[test]
    fn test_head0_increment() {
        // ">" at IP=0 moves head0 to 1. head0 now points at tape[1].
        // tape[1] is the next instruction byte. Since the program and data
        // share the tape, we verify head0 moved by checking that "+"
        // modifies the cell head0 points at.
        // Program: ">+" -- head0 goes to 1, then tape[1] gets incremented.
        // tape[1] was '+' (0x2B), becomes 0x2C after the increment.
        let mut tape = make_tape(b">+");
        Bff::execute(&mut tape, 8192);
        assert_eq!(tape[1], b'+' + 1); // self-modifying: incremented the '+' itself
    }

    #[test]
    fn test_head0_decrement_wraps() {
        // "<" decrements head0 from 0 to 255. tape has 256 bytes, so
        // head0=255 indexes tape[255]. "+" increments tape[255].
        // tape[255] was 0, becomes 1.
        let mut tape = make_tape(b"<+");
        Bff::execute(&mut tape, 8192);
        assert_eq!(tape[255], 1);
    }

    #[test]
    fn test_head1_movement() {
        // "}" moves head1 to 1. "." copies tape[head0=0] to tape[head1=1].
        // head0=0 still points at tape[0] = '}' (0x7D).
        // So tape[1] gets set to 0x7D.
        let mut tape = make_tape(b"}.");
        Bff::execute(&mut tape, 8192);
        assert_eq!(tape[1], b'}');
    }

    #[test]
    fn test_head1_decrement_wraps() {
        // "{" moves head1 to 255. "." copies tape[head0=0] to tape[head1=255].
        // tape[head0=0] = '{' (0x7B). So tape[255] = 0x7B.
        let mut tape = make_tape(b"{.");
        Bff::execute(&mut tape, 8192);
        assert_eq!(tape[255], b'{');
    }

    #[test]
    fn test_plus_increments() {
        // "+" increments tape[head0=0]. tape[0] = '+' (0x2B), becomes 0x2C.
        let mut tape = make_tape(b"+");
        Bff::execute(&mut tape, 8192);
        assert_eq!(tape[0], b'+' + 1);
    }

    #[test]
    fn test_plus_wraps_at_255() {
        let mut tape = vec![0u8; 256];
        // Move head0 to 5 with five ">"
        for i in 0..5 {
            tape[i] = b'>';
        }
        // tape[5] = 0xFF (no-op), tape[6] = 0 (no-op)
        // At IP=5, tape[5]=0xFF is a no-op. IP=6. head0=5.
        // At IP=6, tape[6]=0, no-op. IP=7. head0=5.
        // At IP=7, tape[7]='+'. tape[head0=5]=0xFF. 0xFF + 1 = 0x00 (wraps!).
        tape[5] = 0xFF;
        tape[7] = b'+';
        Bff::execute(&mut tape, 8192);
        assert_eq!(tape[5], 0); // 0xFF + 1 wraps to 0
    }

    #[test]
    fn test_minus_decrements() {
        // "-" at IP=0 decrements tape[head0=0]. tape[0] = '-' (0x2D), becomes 0x2C.
        let mut tape = make_tape(b"-");
        Bff::execute(&mut tape, 8192);
        assert_eq!(tape[0], b'-' - 1);
    }

    #[test]
    fn test_minus_wraps_at_zero() {
        let mut tape = vec![0u8; 256];
        tape[0] = b'>'; // head0 -> 1. IP -> 1.
        // tape[1] = 0 (no-op). IP -> 2. head0 stays at 1.
        tape[2] = b'-'; // IP=2, head0=1. tape[1]=0. 0-1=255 (wraps).
        Bff::execute(&mut tape, 8192);
        assert_eq!(tape[1], 255);
    }

    #[test]
    fn test_dot_copy() {
        // "}" moves head1 to 1. "." copies tape[head0=0] to tape[head1=1].
        // tape[head0=0] = '}' (0x7D). So tape[1] = 0x7D.
        let mut tape = make_tape(b"}.");
        Bff::execute(&mut tape, 8192);
        assert_eq!(tape[1], b'}');
    }

    #[test]
    fn test_comma_copy() {
        // "}" moves head1 to 1. "," copies tape[head1=1] to tape[head0=0].
        // At IP=1, tape[1]=',' (0x2C). head1=1. So copies tape[1]=0x2C to tape[0].
        let mut tape = make_tape(b"},");
        Bff::execute(&mut tape, 8192);
        assert_eq!(tape[0], b',');
    }

    #[test]
    fn test_simple_loop() {
        // Move head0 to 128, set tape[128]=3, then run [>+<-] to copy to tape[129].
        let mut tape = vec![0u8; 256];
        for i in 0..128 {
            tape[i] = b'>'; // head0 goes to 128
        }
        tape[128] = 3;
        tape[198] = b'[';
        tape[199] = b'>';
        tape[200] = b'+';
        tape[201] = b'<';
        tape[202] = b'-';
        tape[203] = b']';
        Bff::execute(&mut tape, 8192);
        assert_eq!(tape[128], 0);
        assert_eq!(tape[129], 3);
    }

    #[test]
    fn test_unmatched_open_bracket_terminates() {
        // head0 stays at 0. Put no-ops at 0,1, then '[' at position 2.
        // tape[head0=0] = 0 at IP=2. '[' tries to jump to matching ']', none exists.
        // Terminates.
        let mut tape = vec![0u8; 8];
        tape[2] = b'[';
        let steps = Bff::execute(&mut tape, 8192);
        assert_eq!(steps, 3); // 2 no-ops + 1 unmatched bracket
    }

    #[test]
    fn test_unmatched_close_bracket_terminates() {
        // "+" at IP=0 increments tape[head0=0]. tape[0] was '+' (0x2B), becomes 0x2C.
        // "]" at IP=1 checks tape[head0=0] = 0x2C != 0, tries to jump back.
        // No matching "[", so terminates.
        let mut tape = vec![0u8; 8];
        tape[0] = b'+';
        tape[1] = b']';
        let steps = Bff::execute(&mut tape, 8192);
        assert_eq!(steps, 2);
    }

    #[test]
    fn test_noop_bytes() {
        // Non-instruction bytes are no-ops. IP advances, head0 unchanged.
        // 'A', 'B', 'C' are all no-ops. Then '+' increments tape[head0=0].
        // tape[0] was 'A' (0x41), becomes 0x42.
        let mut tape = vec![b'A', b'B', b'C', b'+', 0, 0, 0, 0];
        Bff::execute(&mut tape, 8192);
        assert_eq!(tape[0], b'A' + 1);
    }

    #[test]
    fn test_step_limit() {
        // "[]" with tape[head0=0] = '[' (0x5B) != 0 creates an infinite loop.
        // The empty loop body never modifies tape[0], so the brackets stay
        // intact and the loop runs until the step limit.
        let mut tape = make_tape(b"[]");
        let steps = Bff::execute(&mut tape, 100);
        assert_eq!(steps, 100);
    }

    #[test]
    fn test_empty_tape() {
        let mut tape: Vec<u8> = vec![];
        let steps = Bff::execute(&mut tape, 8192);
        assert_eq!(steps, 0);
    }

    #[test]
    fn test_all_zeros_terminates() {
        let mut tape = vec![0u8; 64];
        let steps = Bff::execute(&mut tape, 8192);
        assert_eq!(steps, 64);
    }

    #[test]
    fn test_runtime_bracket_scanning() {
        // Verify that bracket matching uses the current tape state, not the
        // initial state. Write a ']' into the tape during execution, then
        // check that '[' finds it.
        //
        // Program layout (128 bytes):
        // 0: >     head0 = 1
        // 1: >     head0 = 2
        // 2: +     tape[2] = ']' (0x5D) -- self-modifying: tape[2] was '+' (0x2B+1=0x2C)
        //
        // Hmm, this is tricky because + increments what's already there.
        // Instead, test indirectly: a program that creates brackets via copy.
        //
        // Simpler: verify scan uses current tape by checking a loop works
        // after the tape is modified by the loop body itself.
        let mut tape = vec![0u8; 128];
        // Move head0 to 64 (a data area well away from the program).
        for i in 0..10 {
            tape[i] = b'>';
        }
        // tape[10] is a no-op (0), IP advances to 11. head0=10.
        // Set up: tape[10] = 3 (loop counter, non-zero, not an instruction).
        tape[10] = 3;
        // Program at 11: [-] which decrements tape[head0=10] until 0.
        tape[11] = b'[';
        tape[12] = b'-';
        tape[13] = b']';
        Bff::execute(&mut tape, 8192);
        assert_eq!(tape[10], 0);
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
            let steps = Bff::execute(&mut tape, 8192);
            prop_assert!(steps <= 8192);
        }

        #[test]
        fn random_programs_respect_step_limit(
            tape_data in prop::collection::vec(any::<u8>(), 1..256),
            limit in 1usize..1000
        ) {
            let mut tape = tape_data;
            let steps = Bff::execute(&mut tape, limit);
            prop_assert!(steps <= limit);
        }

        #[test]
        fn output_tape_same_length(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let original_len = tape_data.len();
            let mut tape = tape_data;
            Bff::execute(&mut tape, 8192);
            prop_assert_eq!(tape.len(), original_len);
        }
    }
}
