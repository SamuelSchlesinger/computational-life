use crate::substrate::Substrate;

/// The Forth (primordial soup) instruction set from Section 3.1.1 of the paper.
///
/// A stack-based language operating on a byte tape. Instructions are encoded
/// as single bytes with three classes:
/// - `0000 xxxx` (0x00-0x0D): 14 fixed opcodes; 0x0E-0x3F are no-ops
/// - `01xx xxxx` (0x40-0x7F): push low 6 bits as unsigned value
/// - `1Xxx xxxx` (0x80-0xFF): relative jump ±([low 6 bits]+1), bit 6 = sign
///
/// Stack values are u8. Stack underflow returns 0 (allowing the trivial
/// one-byte self-replicator 0x0C to work on an empty stack, per the paper).
/// Stack overflow silently drops the push.
pub struct Forth;

/// Maximum stack depth to prevent unbounded growth.
const MAX_STACK: usize = 256;

/// Fixed-size stack that avoids heap allocation. Stack underflow returns 0.
struct FixedStack {
    data: [u8; MAX_STACK],
    len: usize,
}

impl FixedStack {
    #[inline(always)]
    fn new() -> Self {
        Self {
            data: [0u8; MAX_STACK],
            len: 0,
        }
    }

    #[inline(always)]
    fn push(&mut self, val: u8) {
        if self.len < MAX_STACK {
            self.data[self.len] = val;
            self.len += 1;
        }
    }

    #[inline(always)]
    fn pop(&mut self) -> u8 {
        if self.len > 0 {
            self.len -= 1;
            self.data[self.len]
        } else {
            0
        }
    }

    #[inline(always)]
    fn top(&self) -> u8 {
        if self.len > 0 {
            self.data[self.len - 1]
        } else {
            0
        }
    }

    #[inline(always)]
    fn top_mut(&mut self) -> Option<&mut u8> {
        if self.len > 0 {
            Some(&mut self.data[self.len - 1])
        } else {
            None
        }
    }

    #[inline(always)]
    fn swap_top_two(&mut self) {
        if self.len >= 2 {
            self.data.swap(self.len - 1, self.len - 2);
        }
    }
}

impl Substrate for Forth {
    fn execute(tape: &mut [u8], step_limit: usize) -> usize {
        let len = tape.len();
        if len == 0 {
            return 0;
        }

        let mut stack = FixedStack::new();
        let mut pc: usize = 0;
        let mut steps: usize = 0;

        while pc < len && steps < step_limit {
            steps += 1;
            let instr = tape[pc];

            match instr >> 6 {
                // 00: fixed opcodes (0x00-0x0D valid, rest are no-ops)
                0b00 => {
                    match instr & 0x0F {
                        0x00 if instr < 0x10 => {
                            // READ: <top> = *<top>
                            let top = stack.pop();
                            let addr = top as usize % len;
                            let val = tape[addr];
                            stack.push(val);
                        }
                        0x01 if instr < 0x10 => {
                            // READ64: <top> = *(<top> + 64)
                            let top = stack.pop();
                            let addr = ((top as usize).wrapping_add(64)) % len;
                            let val = tape[addr];
                            stack.push(val);
                        }
                        0x02 if instr < 0x10 => {
                            // WRITE: *<top> = <top-1>; pop; pop
                            let addr_val = stack.pop();
                            let data_val = stack.pop();
                            let addr = addr_val as usize % len;
                            tape[addr] = data_val;
                        }
                        0x03 if instr < 0x10 => {
                            // WRITE64: *(<top> + 64) = <top-1>; pop; pop
                            let addr_val = stack.pop();
                            let data_val = stack.pop();
                            let addr = ((addr_val as usize).wrapping_add(64)) % len;
                            tape[addr] = data_val;
                        }
                        0x04 if instr < 0x10 => {
                            // DUP: push <top>
                            let top = stack.top();
                            stack.push(top);
                        }
                        0x05 if instr < 0x10 => {
                            // POP: discard top
                            stack.pop();
                        }
                        0x06 if instr < 0x10 => {
                            // SWAP: swap <top> and <top-1>
                            stack.swap_top_two();
                        }
                        0x07 if instr < 0x10 => {
                            // SKIPNZ: if <top> != 0: pc++
                            if stack.top() != 0 {
                                pc += 1;
                            }
                        }
                        0x08 if instr < 0x10 => {
                            // INC: <top> = <top> + 1
                            if let Some(top) = stack.top_mut() {
                                *top = top.wrapping_add(1);
                            } else {
                                stack.push(1);
                            }
                        }
                        0x09 if instr < 0x10 => {
                            // DEC: <top> = <top> - 1
                            if let Some(top) = stack.top_mut() {
                                *top = top.wrapping_sub(1);
                            } else {
                                stack.push(255);
                            }
                        }
                        0x0A if instr < 0x10 => {
                            // ADD: <top-1> = <top> + <top-1>; pop
                            let a = stack.pop();
                            if let Some(b) = stack.top_mut() {
                                *b = a.wrapping_add(*b);
                            } else {
                                stack.push(a);
                            }
                        }
                        0x0B if instr < 0x10 => {
                            // SUB: <top-1> = <top> - <top-1>; pop
                            let a = stack.pop();
                            if let Some(b) = stack.top_mut() {
                                *b = a.wrapping_sub(*b);
                            } else {
                                stack.push(a);
                            }
                        }
                        0x0C if instr < 0x10 => {
                            // COPY: *(<top> + 64) = *<top>; pop
                            let addr_val = stack.pop();
                            let src = addr_val as usize % len;
                            let dst = ((addr_val as usize).wrapping_add(64)) % len;
                            tape[dst] = tape[src];
                        }
                        0x0D if instr < 0x10 => {
                            // RCOPY: *<top> = *(<top> + 64); pop
                            let addr_val = stack.pop();
                            let dst = addr_val as usize % len;
                            let src = ((addr_val as usize).wrapping_add(64)) % len;
                            tape[dst] = tape[src];
                        }
                        _ => {
                            // No-op (0x0E-0x0F in low nibble with high bits 00,
                            // or any 0x10-0x3F byte)
                        }
                    }
                }
                // 01: push immediate (low 6 bits as unsigned value)
                0b01 => {
                    let val = instr & 0x3F;
                    stack.push(val);
                }
                // 10 or 11: relative jump
                _ => {
                    let offset = (instr & 0x3F) as usize + 1;
                    if instr & 0x40 == 0 {
                        // bit 6 = 0: forward (positive)
                        pc = pc.wrapping_add(offset);
                    } else {
                        // bit 6 = 1: backward (negative)
                        if offset > pc {
                            break; // would jump before start
                        }
                        pc = pc - offset;
                    }
                    continue; // don't do pc += 1 below
                }
            }

            pc += 1;
        }

        steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a tape of given size with bytes at the start.
    fn make_tape(program: &[u8], size: usize) -> Vec<u8> {
        let mut tape = vec![0u8; size];
        for (i, &b) in program.iter().enumerate() {
            if i < size {
                tape[i] = b;
            }
        }
        tape
    }

    // --- Fixed opcode tests ---

    #[test]
    fn test_read() {
        // Push 10, READ (top becomes tape[10]=42), push 20 (addr), WRITE: tape[20]=42
        let mut tape = make_tape(&[0x4A, 0x00, 0x54, 0x02], 128);
        tape[10] = 42;
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[20], 42);
    }

    #[test]
    fn test_read64() {
        // Push 10, READ64 (top becomes tape[74]=99), push 20 (addr), WRITE: tape[20]=99
        let mut tape = make_tape(&[0x4A, 0x01, 0x54, 0x02], 128);
        tape[74] = 99;
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[20], 99);
    }

    #[test]
    fn test_write() {
        // Push value 42, push addr 30, WRITE: tape[30] = 42
        let mut tape = make_tape(&[0x6A, 0x5E, 0x02], 128);
        // 0x6A = 01_101010 = push 42; 0x5E = 01_011110 = push 30
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[30], 42);
    }

    #[test]
    fn test_write64() {
        // Push value 42, push addr 10, WRITE64: tape[10+64=74] = 42
        let mut tape = make_tape(&[0x6A, 0x4A, 0x03], 128);
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[74], 42);
    }

    #[test]
    fn test_dup() {
        // Push 5, DUP, push addr 20, WRITE: tape[20] = 5
        // Then push addr 21, WRITE: tape[21] = 5 (the DUP'd value)
        let mut tape = make_tape(&[0x45, 0x04, 0x54, 0x02, 0x55, 0x02], 128);
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[20], 5);
        assert_eq!(tape[21], 5);
    }

    #[test]
    fn test_pop() {
        // Push 5, push 10, POP (stack=[5]), push 30 (addr), WRITE: tape[30]=5
        let mut tape = make_tape(&[0x45, 0x4A, 0x05, 0x5E, 0x02], 128);
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[30], 5);
    }

    #[test]
    fn test_swap() {
        // Push 5, push 10, SWAP, push 30, SWAP, WRITE: tape[30] = 10
        // After SWAP of [5,10] -> [10,5]. Push 30 -> [10,5,30]. SWAP -> [10,30,5]. WRITE -> tape[5] = 30.
        // Hmm, let's think differently.
        // Push 5, push 10, SWAP: stack = [10, 5]. Push 20: [10, 5, 20]. WRITE: tape[20] = 5. Stack: [10].
        // push 30: [10, 30]. WRITE: tape[30] = 10.
        let mut tape = make_tape(&[0x45, 0x4A, 0x06, 0x54, 0x02, 0x5E, 0x02], 128);
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[20], 5);
        assert_eq!(tape[30], 10);
    }

    #[test]
    fn test_skipnz() {
        // Push 1, SKIPNZ (skips next), NOP, push 42, push 30, WRITE
        // With skip: push 1 -> SKIPNZ (top=1, skip next) -> skip NOP -> push 42 -> push 30 -> WRITE -> tape[30]=42
        let mut tape = make_tape(&[0x41, 0x07, 0x0F, 0x6A, 0x5E, 0x02], 128);
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[30], 42);

        // Push 0, SKIPNZ (doesn't skip), push 99, push 30, WRITE
        let mut tape = make_tape(&[0x40, 0x07, 0x63, 0x5E, 0x02], 128);
        // 0x40 = push 0, SKIPNZ (0 == 0, no skip), 0x63 = push 35, push 30, WRITE
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[30], 35);
    }

    #[test]
    fn test_inc() {
        // Push 10, INC (top=11), push 20 (addr), WRITE: tape[20]=11
        let mut tape = make_tape(&[0x4A, 0x08, 0x54, 0x02], 128);
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[20], 11);
    }

    #[test]
    fn test_dec() {
        // Push 10, DEC (top=9), push 20 (addr), WRITE: tape[20]=9
        let mut tape = make_tape(&[0x4A, 0x09, 0x54, 0x02], 128);
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[20], 9);
    }

    #[test]
    fn test_inc_wraps() {
        // Build 255: push 63*4 + push 3 + ADD = 255. Then INC -> 0.
        // push 63, push 63, ADD(=126), push 63, ADD(=189), push 63, ADD(=252), push 3, ADD(=255), INC(=0), push 20, WRITE
        let mut tape = make_tape(
            &[0x7F, 0x7F, 0x0A, 0x7F, 0x0A, 0x7F, 0x0A, 0x43, 0x0A, 0x08, 0x54, 0x02],
            128,
        );
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[20], 0);
    }

    #[test]
    fn test_dec_wraps() {
        // Push 0, DEC (top=255), push 20 (addr), WRITE: tape[20]=255
        let mut tape = make_tape(&[0x40, 0x09, 0x54, 0x02], 128);
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[20], 255);
    }

    #[test]
    fn test_add() {
        // Push 10, push 20, ADD (top=30), push 40 (addr), WRITE: tape[40]=30
        let mut tape = make_tape(&[0x4A, 0x54, 0x0A, 0x68, 0x02], 128);
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[40], 30);
    }

    #[test]
    fn test_sub() {
        // Push 10, push 20, SUB (top=20-10=10), push 40 (addr), WRITE: tape[40]=10
        let mut tape = make_tape(&[0x4A, 0x54, 0x0B, 0x68, 0x02], 128);
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[40], 10);
    }

    #[test]
    fn test_copy() {
        // Set tape[10] = 77. Push 10, COPY: tape[10+64=74] = tape[10] = 77.
        let mut tape = make_tape(&[0x4A, 0x0C], 128);
        tape[10] = 77;
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[74], 77);
    }

    #[test]
    fn test_rcopy() {
        // Set tape[74] = 88. Push 10, RCOPY: tape[10] = tape[10+64=74] = 88.
        let mut tape = make_tape(&[0x4A, 0x0D], 128);
        tape[74] = 88;
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[10], 88);
    }

    // --- Push immediate tests ---

    #[test]
    fn test_push_immediate() {
        // Push 42 (0x6A), push 20 (addr), WRITE: tape[20]=42
        let mut tape = make_tape(&[0x6A, 0x54, 0x02], 128);
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[20], 42);
    }

    #[test]
    fn test_push_immediate_zero() {
        // Push 0 (0x40), push 20 (addr), WRITE: tape[20]=0
        let mut tape = make_tape(&[0x40, 0x54, 0x02], 128);
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[20], 0);
    }

    #[test]
    fn test_push_immediate_max() {
        // Push 63 (0x7F), push 20 (addr), WRITE: tape[20]=63
        let mut tape = make_tape(&[0x7F, 0x54, 0x02], 128);
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[20], 63);
    }

    // --- Relative jump tests ---

    #[test]
    fn test_jump_forward() {
        // Jump forward 2 (0x81), skip push 35, push 42, push 20 (addr), WRITE: tape[20]=42
        let mut tape = make_tape(&[0x81, 0x63, 0x6A, 0x54, 0x02], 128);
        // Jump from pc=0 to pc=0+2=2. tape[2]=0x6A=push 42.
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[20], 42);
    }

    #[test]
    fn test_jump_backward() {
        // Set up a simple backward jump loop that runs a few times.
        // push 3, DEC, SKIPNZ, jump-forward(+2 to skip backward-jump), jump-backward(-3)
        // pc:  0     1    2       3                                      4
        // push 3 -> DEC(2) -> SKIPNZ(2!=0, skip pc4) -> jumps to pc5 -> but wait...
        // Actually let me think more carefully.
        // We'll count down from 3 to 0 using a loop:
        // [0] push 3
        // [1] DEC
        // [2] DUP
        // [3] SKIPNZ (if top != 0, skip the forward jump at [4])
        // [4] jump forward +3 (to [7], exit)
        // [5] POP (remove the DUP'd copy)
        // [6] jump backward -5 (to [1])
        // [7] ... (program ends, write result to verify)
        // Wait, SKIPNZ doesn't pop. Let me redesign.
        // push 3 -> stack [3]
        // DEC -> stack [2]
        // DUP -> stack [2, 2]
        // SKIPNZ -> top=2, skip next
        // (skipped: jump forward to exit)
        // POP -> stack [2]
        // jump backward to DEC
        // DEC -> [1], DUP -> [1,1], SKIPNZ skip, (skip fwd), POP -> [1], jump back
        // DEC -> [0], DUP -> [0,0], SKIPNZ no skip, jump fwd to exit
        // At exit: stack = [0, 0]. POP one: [0]. Write to tape[20].

        let mut tape = make_tape(
            &[
                0x43,       // [0] push 3
                0x09,       // [1] DEC
                0x04,       // [2] DUP
                0x07,       // [3] SKIPNZ
                0x82,       // [4] jump forward +3 -> pc 7
                0x05,       // [5] POP
                0xC5,       // [6] jump backward -6 -> pc 0... no, -([5]+1) = -6.
                // Hmm, 0xC5 = 11_000101, bit6=1(backward), low6=5, offset=5+1=6. pc=6-6=0.
                // But we want to jump to [1], so offset should be 5. low6=4. 0xC4.
                // Let me fix:
            ],
            128,
        );
        // Fix: replace jump backward
        tape[6] = 0xC4; // 11_000100, offset=4+1=5, pc=6-5=1
        // At [7]: stack should have [0, 0]. Let's write:
        tape[7] = 0x05; // POP -> [0]
        tape[8] = 0x54; // push 20
        tape[9] = 0x06; // SWAP
        tape[10] = 0x02; // WRITE -> tape[20] = 0

        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[20], 0); // counted down from 3 to 0
    }

    #[test]
    fn test_jump_backward_out_of_bounds_terminates() {
        // Jump backward from pc=0 should terminate.
        let mut tape = make_tape(&[0xC0], 128);
        // 0xC0 = 11_000000, backward, offset=0+1=1. pc=0-1 underflow -> terminate.
        let steps = Forth::execute(&mut tape, 8192);
        assert_eq!(steps, 1);
    }

    // --- Stack underflow (returns 0) tests ---

    #[test]
    fn test_stack_underflow_pop_is_noop() {
        // POP on empty stack is a no-op, execution continues
        let mut tape = make_tape(&[0x05, 0x41, 0x54, 0x02], 128);
        // POP(noop), push 1, push 20, WRITE -> tape[20] = 1
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[20], 1);
    }

    #[test]
    fn test_stack_underflow_read_returns_zero() {
        // READ on empty stack: pops 0, reads tape[0], pushes result
        let mut tape = make_tape(&[0x00, 0x54, 0x02], 128);
        // READ(addr=0 -> tape[0]=0x00), push 20, WRITE -> tape[20] = tape[0]
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[20], 0x00);
    }

    #[test]
    fn test_stack_underflow_add_uses_zero() {
        // Push 5, ADD with only 1 value: pops 5, adds to 0, pushes 5
        let mut tape = make_tape(&[0x45, 0x0A, 0x54, 0x02], 128);
        // push 5, ADD(pop 5, top-1=0 -> 5+0=5), push 20, WRITE -> tape[20] = 5
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[20], 5);
    }

    #[test]
    fn test_stack_underflow_swap_is_noop() {
        // Push 5, SWAP with only 1 value: no-op, stack still [5]
        let mut tape = make_tape(&[0x45, 0x06, 0x54, 0x02], 128);
        // push 5, SWAP(noop), push 20, WRITE -> tape[20] = 5
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[20], 5);
    }

    #[test]
    fn test_stack_underflow_write_uses_zeros() {
        // WRITE on empty stack: addr=0, data=0, writes tape[0]=0
        let mut tape = make_tape(&[0x02], 128);
        tape[0] = 0x02; // WRITE instruction
        Forth::execute(&mut tape, 8192);
        // tape[0] gets overwritten to 0 (data=0)
        assert_eq!(tape[0], 0);
    }

    #[test]
    fn test_stack_overflow_silently_drops() {
        // Overflow doesn't terminate; pushes are silently dropped.
        // push 1, DUP, jump-backward-2 -> loop pushing until overflow, then continues
        let mut tape = make_tape(&[0x41, 0x04, 0xC1], 128);
        // This loops forever (push + DUP each iteration, overflow drops silently)
        let steps = Forth::execute(&mut tape, 10000);
        // Should hit step limit, NOT terminate early
        assert_eq!(steps, 10000);
    }

    // --- Step limit test ---

    #[test]
    fn test_step_limit() {
        // Infinite loop: jump backward to self
        // push 1, SKIPNZ, jump-fwd(skip backward), jump-backward-to-push
        // Simpler: just jump backward by 1 from pc=1: 0x0F(nop), 0xC0(back 1->pc=0)
        let mut tape = make_tape(&[0x0F, 0xC0], 128);
        // pc=0: nop. pc=1: 0xC0 = back 1 -> pc=0. Infinite loop.
        let steps = Forth::execute(&mut tape, 100);
        assert_eq!(steps, 100);
    }

    // --- Trivial self-replicator test ---

    #[test]
    fn test_trivial_self_replicator() {
        // The paper states: "executing 0C on an empty stack will copy itself over
        // onto the first byte of the other string."
        // COPY pops addr (0 from empty stack), copies tape[0] to tape[0+64].
        // In a 128-byte soup tape, tape[64] is the first byte of the second program.
        let mut tape = vec![0u8; 128];
        tape[0] = 0x0C; // COPY instruction, rest is zeros
        Forth::execute(&mut tape, 8192);
        assert_eq!(tape[64], 0x0C); // Copied itself to the second program's first byte
    }

    // --- Edge cases ---

    #[test]
    fn test_empty_tape() {
        let mut tape: Vec<u8> = vec![];
        let steps = Forth::execute(&mut tape, 8192);
        assert_eq!(steps, 0);
    }

    #[test]
    fn test_all_nops() {
        // Bytes 0x0E-0x3F are no-ops
        let mut tape = vec![0x0F; 64];
        let steps = Forth::execute(&mut tape, 8192);
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
            let steps = Forth::execute(&mut tape, 8192);
            prop_assert!(steps <= 8192);
        }

        #[test]
        fn random_programs_respect_step_limit(
            tape_data in prop::collection::vec(any::<u8>(), 1..256),
            limit in 1usize..1000
        ) {
            let mut tape = tape_data;
            let steps = Forth::execute(&mut tape, limit);
            prop_assert!(steps <= limit);
        }

        #[test]
        fn output_tape_same_length(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let original_len = tape_data.len();
            let mut tape = tape_data;
            Forth::execute(&mut tape, 8192);
            prop_assert_eq!(tape.len(), original_len);
        }
    }
}
