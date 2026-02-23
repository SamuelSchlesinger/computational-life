use crate::substrate::Substrate;

/// The EDVAC (Electronic Discrete Variable Automatic Computer, 1951) substrate.
///
/// EDVAC's theoretical design, described in von Neumann's "First Draft of a
/// Report on the EDVAC" (1945), defined the stored-program / von Neumann
/// architecture. The original machine used 44-bit words and mercury delay-line
/// memory.
///
/// The defining feature is the **four-address instruction format**: every
/// instruction specifies two operand addresses (A, B), a result destination
/// (C), and the address of the next instruction (D). There is no implicit
/// program counter increment — the flow of control is always explicit.
///
/// Adaptation for byte tape:
/// - 5-byte instructions: 1 byte opcode + 4 single-byte addresses
/// - Data values are `u8` with wrapping arithmetic
/// - All addresses are taken modulo `tape.len()`
///
/// State:
/// - `pc`: program counter (byte offset), starts at 0
pub struct Edvac;

impl Substrate for Edvac {
    fn execute(tape: &mut [u8], step_limit: usize) -> usize {
        let len = tape.len();
        if len == 0 {
            return 0;
        }

        let mut pc: usize = 0;
        let mut steps: usize = 0;

        while steps < step_limit {
            // Need 5 bytes for a full instruction
            if pc + 4 >= len {
                break;
            }

            steps += 1;

            let opcode = tape[pc] & 0x0F;
            let addr_a = tape[pc + 1] as usize % len;
            let addr_b = tape[pc + 2] as usize % len;
            let addr_c = tape[pc + 3] as usize % len;
            let addr_d = tape[pc + 4] as usize % len;

            match opcode {
                0x0 => {
                    // HALT
                    break;
                }
                0x1 => {
                    // ADD: tape[C] = tape[A] + tape[B]; pc = D
                    tape[addr_c] = tape[addr_a].wrapping_add(tape[addr_b]);
                    pc = addr_d;
                }
                0x2 => {
                    // SUB: tape[C] = tape[A] - tape[B]; pc = D
                    tape[addr_c] = tape[addr_a].wrapping_sub(tape[addr_b]);
                    pc = addr_d;
                }
                0x3 => {
                    // MUL: tape[C] = tape[A] * tape[B]; pc = D
                    tape[addr_c] = tape[addr_a].wrapping_mul(tape[addr_b]);
                    pc = addr_d;
                }
                0x4 => {
                    // DIV: tape[C] = tape[A] / tape[B]; halt if tape[B] == 0
                    if tape[addr_b] == 0 {
                        break;
                    }
                    tape[addr_c] = tape[addr_a] / tape[addr_b];
                    pc = addr_d;
                }
                0x5 => {
                    // AND: tape[C] = tape[A] & tape[B]; pc = D
                    tape[addr_c] = tape[addr_a] & tape[addr_b];
                    pc = addr_d;
                }
                0x6 => {
                    // OR: tape[C] = tape[A] | tape[B]; pc = D
                    tape[addr_c] = tape[addr_a] | tape[addr_b];
                    pc = addr_d;
                }
                0x7 => {
                    // XOR: tape[C] = tape[A] ^ tape[B]; pc = D
                    tape[addr_c] = tape[addr_a] ^ tape[addr_b];
                    pc = addr_d;
                }
                0x8 => {
                    // COPY: tape[C] = tape[A]; B ignored; pc = D
                    tape[addr_c] = tape[addr_a];
                    pc = addr_d;
                }
                0x9 => {
                    // CMP_BR: if tape[A] <= tape[B]: pc = C; else pc = D
                    if tape[addr_a] <= tape[addr_b] {
                        pc = addr_c;
                    } else {
                        pc = addr_d;
                    }
                }
                0xA => {
                    // LOAD_IMM: tape[C] = A (raw address byte as value); pc = D
                    tape[addr_c] = tape[pc + 1];
                    pc = addr_d;
                }
                0xB => {
                    // SHIFT_L: tape[C] = tape[A] << (tape[B] & 0x07); pc = D
                    tape[addr_c] = tape[addr_a] << (tape[addr_b] & 0x07);
                    pc = addr_d;
                }
                0xC => {
                    // SHIFT_R: tape[C] = tape[A] >> (tape[B] & 0x07); pc = D
                    tape[addr_c] = tape[addr_a] >> (tape[addr_b] & 0x07);
                    pc = addr_d;
                }
                0xD => {
                    // NOT: tape[C] = !tape[A]; B ignored; pc = D
                    tape[addr_c] = !tape[addr_a];
                    pc = addr_d;
                }
                0xE => {
                    // MOD: tape[C] = tape[A] % tape[B]; halt if tape[B] == 0
                    if tape[addr_b] == 0 {
                        break;
                    }
                    tape[addr_c] = tape[addr_a] % tape[addr_b];
                    pc = addr_d;
                }
                0xF => {
                    // NOP: pc = D
                    pc = addr_d;
                }
                _ => unreachable!(),
            }
        }

        steps
    }

    fn is_instruction(byte: u8) -> bool {
        // All opcodes 0x0-0xE are "meaningful"; 0xF (NOP) is not
        (byte & 0x0F) <= 0x0E
    }

    fn disassemble(tape: &[u8]) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        let len = tape.len();
        let mut pc = 0;
        while pc < len {
            let b = tape[pc];
            if pc + 4 >= len {
                // Not enough bytes for a full instruction
                let _ = writeln!(out, "{pc:04X}: {b:02X}  <incomplete>");
                pc += 1;
                continue;
            }
            let opcode = b & 0x0F;
            let a = tape[pc + 1];
            let b2 = tape[pc + 2];
            let c = tape[pc + 3];
            let d = tape[pc + 4];
            let desc = match opcode {
                0x0 => format!("HALT  A={a} B={b2} C={c} D={d}"),
                0x1 => format!("ADD   [{c}] = [{a}] + [{b2}]; -> {d}"),
                0x2 => format!("SUB   [{c}] = [{a}] - [{b2}]; -> {d}"),
                0x3 => format!("MUL   [{c}] = [{a}] * [{b2}]; -> {d}"),
                0x4 => format!("DIV   [{c}] = [{a}] / [{b2}]; -> {d}"),
                0x5 => format!("AND   [{c}] = [{a}] & [{b2}]; -> {d}"),
                0x6 => format!("OR    [{c}] = [{a}] | [{b2}]; -> {d}"),
                0x7 => format!("XOR   [{c}] = [{a}] ^ [{b2}]; -> {d}"),
                0x8 => format!("COPY  [{c}] = [{a}]; -> {d}"),
                0x9 => format!("CMP   if [{a}] <= [{b2}]: -> {c}; else -> {d}"),
                0xA => format!("LIMM  [{c}] = #{a}; -> {d}"),
                0xB => format!("SHL   [{c}] = [{a}] << [{b2}]; -> {d}"),
                0xC => format!("SHR   [{c}] = [{a}] >> [{b2}]; -> {d}"),
                0xD => format!("NOT   [{c}] = ~[{a}]; -> {d}"),
                0xE => format!("MOD   [{c}] = [{a}] % [{b2}]; -> {d}"),
                0xF => format!("NOP   -> {d}"),
                _ => unreachable!(),
            };
            let _ = writeln!(
                out,
                "{pc:04X}: {:02X} {:02X} {:02X} {:02X} {:02X}  {desc}",
                tape[pc],
                tape[pc + 1],
                tape[pc + 2],
                tape[pc + 3],
                tape[pc + 4]
            );
            pc += 5;
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

    // Helper: build a 5-byte instruction.
    // opcode uses only low 4 bits.
    fn instr(opcode: u8, a: u8, b: u8, c: u8, d: u8) -> [u8; 5] {
        [opcode & 0x0F, a, b, c, d]
    }

    #[test]
    fn test_empty_tape() {
        let mut tape: Vec<u8> = vec![];
        let steps = Edvac::execute(&mut tape, 8192);
        assert_eq!(steps, 0);
    }

    #[test]
    fn test_halt() {
        // HALT at address 0
        let mut tape = make_tape(&instr(0x0, 0, 0, 0, 0), 64);
        let steps = Edvac::execute(&mut tape, 8192);
        assert_eq!(steps, 1);
    }

    #[test]
    fn test_tape_too_short_for_instruction() {
        // Only 4 bytes — not enough for a 5-byte instruction
        let mut tape = vec![0x01, 0x00, 0x00, 0x00];
        let steps = Edvac::execute(&mut tape, 8192);
        assert_eq!(steps, 0);
    }

    #[test]
    fn test_add() {
        // tape[10] = 3, tape[11] = 7
        // ADD: tape[12] = tape[10] + tape[11] = 10; pc = 5 (next instr)
        // HALT at address 5
        let mut tape = make_tape(&[], 64);
        let i = instr(0x1, 10, 11, 12, 5);
        tape[..5].copy_from_slice(&i);
        let h = instr(0x0, 0, 0, 0, 0);
        tape[5..10].copy_from_slice(&h);
        tape[10] = 3;
        tape[11] = 7;
        Edvac::execute(&mut tape, 8192);
        assert_eq!(tape[12], 10);
    }

    #[test]
    fn test_add_wrapping() {
        let mut tape = make_tape(&[], 64);
        let i = instr(0x1, 10, 11, 12, 5);
        tape[..5].copy_from_slice(&i);
        let h = instr(0x0, 0, 0, 0, 0);
        tape[5..10].copy_from_slice(&h);
        tape[10] = 200;
        tape[11] = 100;
        Edvac::execute(&mut tape, 8192);
        assert_eq!(tape[12], 200u8.wrapping_add(100)); // 44
    }

    #[test]
    fn test_sub() {
        let mut tape = make_tape(&[], 64);
        let i = instr(0x2, 10, 11, 12, 5);
        tape[..5].copy_from_slice(&i);
        let h = instr(0x0, 0, 0, 0, 0);
        tape[5..10].copy_from_slice(&h);
        tape[10] = 20;
        tape[11] = 7;
        Edvac::execute(&mut tape, 8192);
        assert_eq!(tape[12], 13);
    }

    #[test]
    fn test_sub_wrapping() {
        let mut tape = make_tape(&[], 64);
        let i = instr(0x2, 10, 11, 12, 5);
        tape[..5].copy_from_slice(&i);
        let h = instr(0x0, 0, 0, 0, 0);
        tape[5..10].copy_from_slice(&h);
        tape[10] = 3;
        tape[11] = 10;
        Edvac::execute(&mut tape, 8192);
        assert_eq!(tape[12], 3u8.wrapping_sub(10)); // 249
    }

    #[test]
    fn test_copy() {
        let mut tape = make_tape(&[], 64);
        let i = instr(0x8, 10, 0, 12, 5);
        tape[..5].copy_from_slice(&i);
        let h = instr(0x0, 0, 0, 0, 0);
        tape[5..10].copy_from_slice(&h);
        tape[10] = 42;
        Edvac::execute(&mut tape, 8192);
        assert_eq!(tape[12], 42);
    }

    #[test]
    fn test_load_imm() {
        // LOAD_IMM: tape[C] = A (raw byte value)
        let mut tape = make_tape(&[], 64);
        let i = instr(0xA, 99, 0, 12, 5);
        tape[..5].copy_from_slice(&i);
        let h = instr(0x0, 0, 0, 0, 0);
        tape[5..10].copy_from_slice(&h);
        Edvac::execute(&mut tape, 8192);
        assert_eq!(tape[12], 99);
    }

    #[test]
    fn test_cmp_br_taken() {
        // tape[10] = 3, tape[11] = 5. 3 <= 5, so pc = C = 15.
        // HALT at address 15.
        let mut tape = make_tape(&[], 64);
        let i = instr(0x9, 10, 11, 15, 20);
        tape[..5].copy_from_slice(&i);
        let h = instr(0x0, 0, 0, 0, 0);
        tape[15..20].copy_from_slice(&h);
        tape[20..25].copy_from_slice(&instr(0xA, 77, 0, 30, 15)); // would set tape[30]=77
        tape[10] = 3;
        tape[11] = 5;
        let steps = Edvac::execute(&mut tape, 8192);
        assert_eq!(steps, 2); // CMP_BR + HALT
        assert_eq!(tape[30], 0); // the LOAD_IMM at addr 20 was never reached
    }

    #[test]
    fn test_cmp_br_not_taken() {
        // tape[10] = 8, tape[11] = 5. 8 > 5, so pc = D = 20.
        // HALT at address 20.
        let mut tape = make_tape(&[], 64);
        let i = instr(0x9, 10, 11, 15, 20);
        tape[..5].copy_from_slice(&i);
        let h_c = instr(0xA, 77, 0, 30, 20); // at 15: would set tape[30]=77
        tape[15..20].copy_from_slice(&h_c);
        let h = instr(0x0, 0, 0, 0, 0);
        tape[20..25].copy_from_slice(&h);
        tape[10] = 8;
        tape[11] = 5;
        let steps = Edvac::execute(&mut tape, 8192);
        assert_eq!(steps, 2); // CMP_BR + HALT
        assert_eq!(tape[30], 0); // the LOAD_IMM at addr 15 was never reached
    }

    #[test]
    fn test_cmp_br_equal() {
        // tape[10] = 5, tape[11] = 5. 5 <= 5, so pc = C = 15.
        let mut tape = make_tape(&[], 64);
        let i = instr(0x9, 10, 11, 15, 20);
        tape[..5].copy_from_slice(&i);
        let h = instr(0x0, 0, 0, 0, 0);
        tape[15..20].copy_from_slice(&h);
        tape[10] = 5;
        tape[11] = 5;
        let steps = Edvac::execute(&mut tape, 8192);
        assert_eq!(steps, 2); // CMP_BR -> HALT at 15
    }

    #[test]
    fn test_four_address_jump() {
        // Verify that D (fourth address) controls the next pc.
        // Instruction at 0: ADD tape[10]+tape[11]->tape[12], then jump to 20.
        // Instruction at 20: HALT.
        // Bytes 5..19 should not be executed.
        let mut tape = make_tape(&[], 64);
        tape[..5].copy_from_slice(&instr(0x1, 10, 11, 12, 20));
        tape[5..10].copy_from_slice(&instr(0xA, 99, 0, 13, 20)); // should be skipped
        tape[20..25].copy_from_slice(&instr(0x0, 0, 0, 0, 0));
        tape[10] = 1;
        tape[11] = 2;
        Edvac::execute(&mut tape, 8192);
        assert_eq!(tape[12], 3); // ADD executed
        assert_eq!(tape[13], 0); // LOAD_IMM at 5 was skipped
    }

    #[test]
    fn test_step_limit() {
        // NOP that jumps to itself: infinite loop
        let mut tape = make_tape(&instr(0xF, 0, 0, 0, 0), 64);
        let steps = Edvac::execute(&mut tape, 100);
        assert_eq!(steps, 100);
    }

    #[test]
    fn test_mul() {
        let mut tape = make_tape(&[], 64);
        tape[..5].copy_from_slice(&instr(0x3, 10, 11, 12, 5));
        tape[5..10].copy_from_slice(&instr(0x0, 0, 0, 0, 0));
        tape[10] = 6;
        tape[11] = 7;
        Edvac::execute(&mut tape, 8192);
        assert_eq!(tape[12], 42);
    }

    #[test]
    fn test_div() {
        let mut tape = make_tape(&[], 64);
        tape[..5].copy_from_slice(&instr(0x4, 10, 11, 12, 5));
        tape[5..10].copy_from_slice(&instr(0x0, 0, 0, 0, 0));
        tape[10] = 42;
        tape[11] = 7;
        Edvac::execute(&mut tape, 8192);
        assert_eq!(tape[12], 6);
    }

    #[test]
    fn test_div_by_zero_halts() {
        let mut tape = make_tape(&[], 64);
        tape[..5].copy_from_slice(&instr(0x4, 10, 11, 12, 5));
        tape[5..10].copy_from_slice(&instr(0xA, 99, 0, 13, 10)); // should not execute
        tape[10] = 42;
        tape[11] = 0;
        let steps = Edvac::execute(&mut tape, 8192);
        assert_eq!(steps, 1);
        assert_eq!(tape[12], 0); // result was not written
        assert_eq!(tape[13], 0); // next instr was not executed
    }

    #[test]
    fn test_mod() {
        let mut tape = make_tape(&[], 64);
        tape[..5].copy_from_slice(&instr(0xE, 10, 11, 12, 5));
        tape[5..10].copy_from_slice(&instr(0x0, 0, 0, 0, 0));
        tape[10] = 17;
        tape[11] = 5;
        Edvac::execute(&mut tape, 8192);
        assert_eq!(tape[12], 2);
    }

    #[test]
    fn test_mod_by_zero_halts() {
        let mut tape = make_tape(&[], 64);
        tape[..5].copy_from_slice(&instr(0xE, 10, 11, 12, 5));
        tape[10] = 17;
        tape[11] = 0;
        let steps = Edvac::execute(&mut tape, 8192);
        assert_eq!(steps, 1);
    }

    #[test]
    fn test_and() {
        let mut tape = make_tape(&[], 64);
        tape[..5].copy_from_slice(&instr(0x5, 10, 11, 12, 5));
        tape[5..10].copy_from_slice(&instr(0x0, 0, 0, 0, 0));
        tape[10] = 0b11001100;
        tape[11] = 0b10101010;
        Edvac::execute(&mut tape, 8192);
        assert_eq!(tape[12], 0b10001000);
    }

    #[test]
    fn test_or() {
        let mut tape = make_tape(&[], 64);
        tape[..5].copy_from_slice(&instr(0x6, 10, 11, 12, 5));
        tape[5..10].copy_from_slice(&instr(0x0, 0, 0, 0, 0));
        tape[10] = 0b11001100;
        tape[11] = 0b10101010;
        Edvac::execute(&mut tape, 8192);
        assert_eq!(tape[12], 0b11101110);
    }

    #[test]
    fn test_xor() {
        let mut tape = make_tape(&[], 64);
        tape[..5].copy_from_slice(&instr(0x7, 10, 11, 12, 5));
        tape[5..10].copy_from_slice(&instr(0x0, 0, 0, 0, 0));
        tape[10] = 0b11001100;
        tape[11] = 0b10101010;
        Edvac::execute(&mut tape, 8192);
        assert_eq!(tape[12], 0b01100110);
    }

    #[test]
    fn test_not() {
        let mut tape = make_tape(&[], 64);
        tape[..5].copy_from_slice(&instr(0xD, 10, 0, 12, 5));
        tape[5..10].copy_from_slice(&instr(0x0, 0, 0, 0, 0));
        tape[10] = 0b11001100;
        Edvac::execute(&mut tape, 8192);
        assert_eq!(tape[12], 0b00110011);
    }

    #[test]
    fn test_shift_left() {
        let mut tape = make_tape(&[], 64);
        tape[..5].copy_from_slice(&instr(0xB, 10, 11, 12, 5));
        tape[5..10].copy_from_slice(&instr(0x0, 0, 0, 0, 0));
        tape[10] = 0b00000011;
        tape[11] = 3;
        Edvac::execute(&mut tape, 8192);
        assert_eq!(tape[12], 0b00011000);
    }

    #[test]
    fn test_shift_right() {
        let mut tape = make_tape(&[], 64);
        tape[..5].copy_from_slice(&instr(0xC, 10, 11, 12, 5));
        tape[5..10].copy_from_slice(&instr(0x0, 0, 0, 0, 0));
        tape[10] = 0b11000000;
        tape[11] = 3;
        Edvac::execute(&mut tape, 8192);
        assert_eq!(tape[12], 0b00011000);
    }

    #[test]
    fn test_nop() {
        // NOP at 0 jumps to 5; HALT at 5.
        let mut tape = make_tape(&[], 64);
        tape[..5].copy_from_slice(&instr(0xF, 0, 0, 0, 5));
        tape[5..10].copy_from_slice(&instr(0x0, 0, 0, 0, 0));
        let steps = Edvac::execute(&mut tape, 8192);
        assert_eq!(steps, 2);
    }

    #[test]
    fn test_address_wrapping() {
        // Addresses wrap modulo tape.len()
        let size = 32;
        let mut tape = make_tape(&[], size);
        // LOAD_IMM: tape[C=40%32=8] = A=55; then HALT at D=5
        tape[..5].copy_from_slice(&instr(0xA, 55, 0, 40, 5));
        tape[5..10].copy_from_slice(&instr(0x0, 0, 0, 0, 0));
        Edvac::execute(&mut tape, 8192);
        assert_eq!(tape[8], 55); // 40 % 32 = 8
    }

    #[test]
    fn test_is_instruction() {
        for b in 0..=255u8 {
            let low = b & 0x0F;
            if low <= 0x0E {
                assert!(Edvac::is_instruction(b));
            } else {
                assert!(!Edvac::is_instruction(b));
            }
        }
    }

    #[test]
    fn test_disassemble() {
        let mut tape = make_tape(&[], 10);
        tape[..5].copy_from_slice(&instr(0x1, 10, 11, 12, 5));
        tape[5..10].copy_from_slice(&instr(0x0, 0, 0, 0, 0));
        let dis = Edvac::disassemble(&tape);
        assert!(dis.contains("ADD"));
        assert!(dis.contains("HALT"));
    }

    #[test]
    fn test_chained_instructions() {
        // Two ADDs chained via the four-address mechanism:
        // Instr at 0: ADD tape[20]+tape[21] -> tape[22], then -> 5
        // Instr at 5: ADD tape[22]+tape[23] -> tape[24], then -> 10
        // Instr at 10: HALT
        let mut tape = make_tape(&[], 64);
        tape[..5].copy_from_slice(&instr(0x1, 20, 21, 22, 5));
        tape[5..10].copy_from_slice(&instr(0x1, 22, 23, 24, 10));
        tape[10..15].copy_from_slice(&instr(0x0, 0, 0, 0, 0));
        tape[20] = 10;
        tape[21] = 20;
        tape[23] = 5;
        Edvac::execute(&mut tape, 8192);
        assert_eq!(tape[22], 30); // 10 + 20
        assert_eq!(tape[24], 35); // 30 + 5
    }

    #[test]
    fn test_self_modifying_code() {
        // LOAD_IMM to overwrite an opcode, demonstrating self-modifying code.
        // Instr at 0: LOAD_IMM tape[5] = 0x00 (change opcode at addr 5 to HALT), -> 5
        // Instr at 5: originally NOP -> 5 (infinite loop), but gets changed to HALT
        let mut tape = make_tape(&[], 64);
        tape[..5].copy_from_slice(&instr(0xA, 0x00, 0, 5, 5));
        tape[5..10].copy_from_slice(&instr(0xF, 0, 0, 0, 5)); // NOP loop
        Edvac::execute(&mut tape, 8192);
        // After LOAD_IMM, tape[5] = 0x00, so the instruction at 5 becomes HALT.
        assert_eq!(tape[5], 0x00);
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
            let steps = Edvac::execute(&mut tape, 8192);
            prop_assert!(steps <= 8192);
        }

        #[test]
        fn random_programs_respect_step_limit(
            tape_data in prop::collection::vec(any::<u8>(), 1..256),
            limit in 1usize..1000
        ) {
            let mut tape = tape_data;
            let steps = Edvac::execute(&mut tape, limit);
            prop_assert!(steps <= limit);
        }

        #[test]
        fn output_tape_same_length(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let original_len = tape_data.len();
            let mut tape = tape_data;
            Edvac::execute(&mut tape, 8192);
            prop_assert_eq!(tape.len(), original_len);
        }
    }
}
