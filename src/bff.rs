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

        // Pre-compute bracket match table.
        // bracket_match[i] = index of matching bracket, or usize::MAX if unmatched.
        let bracket_match = build_bracket_table(tape);

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
                        let target = bracket_match[ip];
                        if target == usize::MAX {
                            // Unmatched bracket: terminate
                            break;
                        }
                        ip = target;
                        continue;
                    }
                }
                RBRACKET => {
                    let idx = head0 as usize % len;
                    if tape[idx] != 0 {
                        let target = bracket_match[ip];
                        if target == usize::MAX {
                            // Unmatched bracket: terminate
                            break;
                        }
                        ip = target;
                        continue;
                    }
                }
                _ => {} // no-op
            }
            ip += 1;
        }

        steps
    }
}

/// Build a bracket-match lookup table for the tape.
///
/// Returns a Vec where `result[i]` is the index of the matching bracket
/// for position `i`, or `usize::MAX` if the bracket at `i` is unmatched
/// (or if position `i` is not a bracket).
fn build_bracket_table(tape: &[u8]) -> Vec<usize> {
    let mut table = vec![usize::MAX; tape.len()];
    let mut stack = Vec::new();

    for i in 0..tape.len() {
        match tape[i] {
            LBRACKET => {
                stack.push(i);
            }
            RBRACKET => {
                if let Some(open) = stack.pop() {
                    table[open] = i;
                    table[i] = open;
                }
                // If stack is empty, bracket remains usize::MAX (unmatched)
            }
            _ => {}
        }
    }
    // Any remaining items on the stack are unmatched '[', already usize::MAX

    table
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
        // Move head0 to a cell containing 255, then "+".
        // Use a 256-byte tape. Set tape[128]=255. Program moves head0 there.
        let mut tape = vec![0u8; 256];
        // 128 ">" instructions to move head0 to 128
        for i in 0..128 {
            tape[i] = b'>';
        }
        tape[128] = b'+'; // this is also an instruction! IP=128, head0=128.
        // tape[128] = '+' = 0x2B. After "+", tape[128] = 0x2C.
        // That's not 255->0 wrapping. Let's use a different approach.
        // Set tape[129] = 255. Program: 128 ">" + ">+" to move to 129 and increment.
        for i in 0..129 {
            tape[i] = b'>';
        }
        tape[129] = b'+';
        tape[130] = 0; // no more program
        // After 129 ">", head0=129. IP=129, tape[129]='+', tape[head0=129] += 1.
        // But tape[129] = '+' = 0x2B, not 255. We need the DATA to be 255.
        // The problem: the instruction itself IS the data at that position.
        // Solution: put data AFTER the program ends.
        for i in 0..129 {
            tape[i] = b'>';
        }
        tape[129] = b'+'; // instruction at position 129
        // head0=129 when IP reaches 129. tape[129]='+' (0x2B) gets incremented to 0x2C.
        // We can't easily separate program from data in a minimal test.
        // Instead, test wrapping by setting up a value and checking.
        let mut tape = vec![0u8; 256];
        tape[0] = b'+'; // IP=0: tape[head0=0] = 0x2B + 1 = 0x2C
        tape[0] = 0xFF; // But 0xFF is a no-op! We can't have 255 be the instruction.
        // Let's just verify wrapping arithmetic directly: put 0xFF at a data cell.
        let mut tape = vec![0u8; 256];
        for i in 0..200 {
            tape[i] = b'>';
        }
        tape[200] = b'+';
        tape[201] = 0; // end
        // head0=200 at IP=200. But tape[200] = '+'. The + instruction increments
        // tape[head0=200] = tape[200] = 0x2B + 1 = 0x2C. Not what we want.
        // We fundamentally can't put 255 at a position and then run + on it
        // without the instruction being at a different position.
        // Fix: move head0 to 201 (which is 0), set it to 255, then +.
        let mut tape = vec![0u8; 256];
        for i in 0..201 {
            tape[i] = b'>';
        }
        tape[201] = b'+';
        // head0 = 201 at IP=201. tape[201] = '+' = 0x2B. After +, tape[201] = 0x2C. Nope.
        // The crux: the instruction at tape[ip] and the data at tape[head0] are the SAME
        // when head0 == ip. We need head0 != ip.
        // Use "<" to move head0 backwards: ">>>>>...>>>><+" (move head0 forward a lot,
        // then back one, so head0 != ip when we hit the +).
        // Actually simpler: the paper's BFF has head0 as a separate pointer.
        // head0 doesn't track IP. So after N ">" instructions, head0=N but IP=N too.
        // We just need one more ">" to desync them: nah, that makes head0=N+1 and IP=N+1.
        // Key insight: head0 only changes on <>. IP always advances (except brackets).
        // A no-op advances IP but NOT head0. So: put no-ops between > and +.
        let mut tape = vec![0u8; 256];
        // Move head0 to 5 with five ">"
        for i in 0..5 {
            tape[i] = b'>';
        }
        // tape[5] = 0 (no-op), IP=5 -> IP=6. head0 stays at 5.
        // tape[6] = 0 (no-op), IP=6 -> IP=7. head0 stays at 5.
        tape[7] = b'+'; // IP=7, head0=5. tape[5] was 0 (no-op byte), becomes 1.
        // But we want it to be 255. Set tape[5] to something...
        // If tape[5] is a no-op byte, it won't affect head0. Let's set it to 0xFF.
        // 0xFF is not a BFF instruction, so it's a no-op. head0 stays.
        // But the ">" at positions 0-4 set head0 to 5, IP to 5.
        // At IP=5, tape[5]=0xFF is a no-op. IP=6. head0=5.
        // At IP=6, tape[6]=0, no-op. IP=7. head0=5.
        // At IP=7, tape[7]='+'. tape[head0=5]=0xFF. 0xFF + 1 = 0x00 (wraps!). IP=8.
        tape[5] = 0xFF;
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
        // Similar trick: move head0 to a zero cell via ">", then use no-ops
        // to advance IP past that cell before "-".
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
        // Use a 256-byte tape. Put program at start, data area at positions 200+.
        // Program: move head0 to 200, then loop [>+<-] to copy tape[200] to tape[201].
        let mut tape = vec![0u8; 256];
        // 200 ">" to move head0 to 200
        for i in 0..200 {
            tape[i] = b'>';
        }
        // tape[200..205] = "[>+<-]" but IP is at 200 and head0 is at 200.
        // Problem: tape[200] = '[' and head0=200, so tape[head0]=tape[200]='[' (0x5B) != 0.
        // The loop won't be entered for jumping, it'll just proceed (since value != 0
        // means [ doesn't jump forward). That's actually what we want for the first iteration.
        //
        // Actually, we need head0 to point at a data cell, not the instruction cell.
        // Let's put one more ">" so head0=201, and put data at 201.
        // Program: 201 ">" then [>+<-] starting at tape[201].
        // But head0=201 and IP=201. Same issue.
        //
        // Alternative: put no-ops between the ">" chain and the loop, to desync IP and head0.
        // Move head0 to 128 with 128 ">", then 70 no-ops (to advance IP to 198),
        // then the loop program [>+<-] at positions 198-203.
        // At IP=198, head0=128. tape[128]=0 initially. [ checks tape[head0=128]=0, jumps to ].
        // That's an empty loop. We need tape[128] to be non-zero.
        // Set tape[128] = 3 (a no-op byte), then the loop decrements it.
        let mut tape = vec![0u8; 256];
        for i in 0..128 {
            tape[i] = b'>'; // head0 goes to 128
        }
        // IP=128..197 are no-ops (0), head0 stays at 128
        // Set data: tape[128] will be read at IP=128 as a no-op (it's 3, which is no-op).
        tape[128] = 3;
        // tape[129] = 0 (accumulator destination after > in loop)
        // Program loop at position 198: [>+<-]
        tape[198] = b'[';
        tape[199] = b'>';
        tape[200] = b'+';
        tape[201] = b'<';
        tape[202] = b'-';
        tape[203] = b']';
        // At IP=198: '['. tape[head0=128]=3 != 0. Don't jump. IP=199.
        // IP=199: '>'. head0=129. IP=200.
        // IP=200: '+'. tape[129] += 1 -> 1. IP=201.
        // IP=201: '<'. head0=128. IP=202.
        // IP=202: '-'. tape[128] = 3-1 = 2. IP=203.
        // IP=203: ']'. tape[head0=128]=2 != 0. Jump to IP=198.
        // ... repeats until tape[128]=0. tape[129] should be 3.
        Bff::execute(&mut tape, 8192);
        assert_eq!(tape[128], 0);
        assert_eq!(tape[129], 3);
    }

    #[test]
    fn test_unmatched_open_bracket_terminates() {
        // Put an unmatched [ where tape[head0] == 0.
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
    fn test_nested_brackets_table() {
        let tape = b"[[]]";
        let table = build_bracket_table(tape);
        assert_eq!(table[0], 3);
        assert_eq!(table[1], 2);
        assert_eq!(table[2], 1);
        assert_eq!(table[3], 0);
    }

    #[test]
    fn test_bracket_table_unmatched() {
        let tape = b"[[]";
        let table = build_bracket_table(tape);
        assert_eq!(table[0], usize::MAX);
        assert_eq!(table[1], 2);
        assert_eq!(table[2], 1);
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
        // "[+]" with tape[head0=0] = '[' (0x5B) != 0 creates an infinite loop.
        let mut tape = make_tape(b"[+]");
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
