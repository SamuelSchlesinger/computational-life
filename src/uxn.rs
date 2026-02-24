use std::panic::{AssertUnwindSafe, catch_unwind};

use baryuxn::execute_operation;
use baryuxn::prelude::*;

use crate::substrate::Substrate;

/// Wraps a byte-slice tape as Uxn memory with modular addressing.
///
/// The blanket `UxnMemory` impl on `&mut [u8]` would panic on addresses
/// beyond the slice length, so we wrap with modular arithmetic.
struct TapeMemory<'a> {
    tape: &'a mut [u8],
}

impl UxnMemory for TapeMemory<'_> {
    fn get_memory(&self, address: u16) -> u8 {
        if self.tape.is_empty() {
            return 0;
        }
        self.tape[address as usize % self.tape.len()]
    }

    fn get_memory_mut(&mut self, address: u16) -> &mut u8 {
        let len = self.tape.len();
        if len == 0 {
            panic!("TapeMemory::get_memory_mut called on empty tape");
        }
        &mut self.tape[address as usize % len]
    }
}

/// No-op device bus — Uxn programs in our simulation never perform I/O.
struct NullBus;

impl UxnDeviceBus for NullBus {
    fn read(&mut self, _machine: &mut UxnMachineState, _address: u8) -> u8 {
        0
    }

    fn write(&mut self, _machine: &mut UxnMachineState, _address: u8, _byte: u8) {}
}

/// The Uxn stack machine (Hundred Rabbits / Varvara).
///
/// All 256 byte values decode to valid operations, giving 100% instruction
/// density — every random byte does *something*. The dual circular stacks
/// (work + return, each 256 bytes) wrap on overflow rather than trapping,
/// making stack-heavy programs robust on random tapes.
///
/// BRK (0x00) halts execution, acting as the natural termination signal.
pub struct Uxn;

impl Substrate for Uxn {
    fn execute(tape: &mut [u8], step_limit: usize) -> usize {
        if tape.is_empty() {
            return 0;
        }
        let mut state = UxnMachineState::new();
        let mut memory = TapeMemory { tape };
        let mut bus = NullBus;
        let mut pc: u16 = 0;
        let mut steps = 0;

        while steps < step_limit {
            // baryuxn's SFT opcode can panic on shift amounts >= 8 in byte
            // mode (Rust overflow check). Treat such panics as halts.
            let result = catch_unwind(AssertUnwindSafe(|| {
                execute_operation(&mut state, pc, &mut memory, &mut bus)
            }));
            steps += 1;
            match result {
                Ok((_opcode, Some(npc))) => pc = npc,
                Ok((_opcode, None)) => break, // BRK — halt
                Err(_) => break,              // panic in crate — treat as halt
            }
        }
        steps
    }

    fn execute_battle(tape: &mut [u8], ps: usize, step_limit: usize) -> usize {
        if tape.is_empty() {
            return 0;
        }
        let mut state_a = UxnMachineState::new();
        let mut state_b = UxnMachineState::new();
        let mut pc_a: u16 = 0;
        let mut pc_b: u16 = ps as u16;
        let mut bus = NullBus;

        let mut steps = 0;
        let mut halted_a = false;
        let mut halted_b = false;

        while steps < step_limit && (!halted_a || !halted_b) {
            if !halted_a {
                let mut memory = TapeMemory { tape };
                let result = catch_unwind(AssertUnwindSafe(|| {
                    execute_operation(&mut state_a, pc_a, &mut memory, &mut bus)
                }));
                steps += 1;
                match result {
                    Ok((_opcode, Some(npc))) => pc_a = npc,
                    Ok((_, None)) | Err(_) => halted_a = true,
                }
                if steps >= step_limit {
                    break;
                }
            }
            if !halted_b {
                let mut memory = TapeMemory { tape };
                let result = catch_unwind(AssertUnwindSafe(|| {
                    execute_operation(&mut state_b, pc_b, &mut memory, &mut bus)
                }));
                steps += 1;
                match result {
                    Ok((_opcode, Some(npc))) => pc_b = npc,
                    Ok((_, None)) | Err(_) => halted_b = true,
                }
            }
        }
        steps
    }

    fn is_instruction(_byte: u8) -> bool {
        true // All 256 values are valid Uxn opcodes
    }

    fn disassemble(tape: &[u8]) -> String {
        use std::fmt::Write;
        if tape.is_empty() {
            return String::new();
        }
        let mut out = String::new();
        let mut addr = 0usize;
        while addr < tape.len() {
            let opcode = tape[addr];
            let (base_mnemonic, operand_bytes) = disasm_opcode(opcode);
            let mnemonic = format!("{}{}", base_mnemonic, mode_suffix(opcode));
            // Collect hex bytes
            let mut hex = format!("{:02X}", opcode);
            for i in 1..=operand_bytes {
                let idx = (addr + i) % tape.len();
                write!(hex, " {:02X}", tape[idx]).unwrap();
            }
            // Format operand
            let operand = if operand_bytes == 1 {
                let idx = (addr + 1) % tape.len();
                format!(" ${:02X}", tape[idx])
            } else if operand_bytes == 2 {
                let idx1 = (addr + 1) % tape.len();
                let idx2 = (addr + 2) % tape.len();
                format!(" ${:02X}{:02X}", tape[idx1], tape[idx2])
            } else {
                String::new()
            };
            let _ = writeln!(out, "{addr:04X}: {hex:<12} {mnemonic}{operand}");
            addr += 1 + operand_bytes;
        }
        out
    }
}

/// Base opcode mnemonics (bits 0-4, indices 0x01..=0x1f).
const BASE_MNEMONICS: [&str; 31] = [
    "INC", "POP", "NIP", "SWP", "ROT", "DUP", "OVR", "EQU", "NEQ", "GTH", "LTH", "JMP", "JCN",
    "JSR", "STH", "LDZ", "STZ", "LDR", "STR", "LDA", "STA", "DEI", "DEO", "ADD", "SUB", "MUL",
    "DIV", "AND", "ORA", "EOR", "SFT",
];

/// Format the mode suffix for a non-special opcode (base != 0).
/// Returns "" for special opcodes (BRK, JCI, JMI, JSI, LIT variants).
fn mode_suffix(opcode: u8) -> &'static str {
    if opcode & 0x1f == 0 {
        return "";
    }
    // bit 5 = short(2), bit 7 = keep(k), bit 6 = return(r)
    match (opcode & 0x20 != 0, opcode & 0x80 != 0, opcode & 0x40 != 0) {
        (false, false, false) => "",
        (true, false, false) => "2",
        (false, true, false) => "k",
        (false, false, true) => "r",
        (true, true, false) => "2k",
        (true, false, true) => "2r",
        (false, true, true) => "kr",
        (true, true, true) => "2kr",
    }
}

/// Decode a Uxn opcode into its mnemonic and the number of immediate operand bytes.
///
/// The encoding is very regular:
/// - 0x00 = BRK, 0x20 = JCI, 0x40 = JMI, 0x60 = JSI (special family)
/// - 0x80/0xa0/0xc0/0xe0 = LIT/LIT2/LITr/LIT2r (consume 1 or 2 immediate bytes)
/// - All other bytes: bits 0-4 = base opcode, bit 5 = short(2), bit 6 = return(r), bit 7 = keep(k)
fn disasm_opcode(opcode: u8) -> (&'static str, usize) {
    match opcode {
        0x00 => ("BRK", 0),
        0x20 => ("JCI", 2),
        0x40 => ("JMI", 2),
        0x60 => ("JSI", 2),
        0x80 => ("LIT", 1),
        0xa0 => ("LIT2", 2),
        0xc0 => ("LITr", 1),
        0xe0 => ("LIT2r", 2),
        _ => {
            let base = (opcode & 0x1f) as usize;
            if base == 0 {
                ("???", 0)
            } else {
                (BASE_MNEMONICS[base - 1], 0)
            }
        }
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
    fn brk() {
        let mut tape = make_tape(&[0x00], 16);
        let steps = Uxn::execute(&mut tape, 256);
        assert_eq!(steps, 1);
    }

    #[test]
    fn lit() {
        // LIT 0x42 pushes 0x42 onto the work stack; then BRK halts.
        let mut tape = make_tape(&[0x80, 0x42, 0x00], 16);
        let steps = Uxn::execute(&mut tape, 256);
        assert_eq!(steps, 2);
    }

    #[test]
    fn lit2() {
        // LIT2 pushes a 16-bit short; then BRK halts.
        let mut tape = make_tape(&[0xa0, 0x12, 0x34, 0x00], 16);
        let steps = Uxn::execute(&mut tape, 256);
        assert_eq!(steps, 2);
    }

    #[test]
    fn add() {
        // LIT 0x03, LIT 0x04, ADD, BRK
        let mut tape = make_tape(&[0x80, 0x03, 0x80, 0x04, 0x18, 0x00], 16);
        let steps = Uxn::execute(&mut tape, 256);
        assert_eq!(steps, 4);
    }

    #[test]
    fn sta() {
        // STA pops address (top), then value (below).
        // Push value first, then address: LIT 0xAB, LIT2 0x0008, STA, BRK
        let mut tape = make_tape(&[0x80, 0xAB, 0xa0, 0x00, 0x08, 0x15, 0x00], 16);
        Uxn::execute(&mut tape, 256);
        assert_eq!(tape[8], 0xAB);
    }

    #[test]
    fn lda() {
        // LDA pops address, pushes value. Then STA stores it elsewhere.
        // LIT2 0x0008, LDA, LIT2 0x000A, STA, BRK
        let mut tape = make_tape(&[0xa0, 0x00, 0x08, 0x14, 0xa0, 0x00, 0x0A, 0x15, 0x00], 16);
        tape[8] = 0xEE;
        Uxn::execute(&mut tape, 256);
        assert_eq!(tape[0x0A], 0xEE);
    }

    #[test]
    fn jmp() {
        // JMP2 pops absolute address from stack.
        // LIT2 0x0006, JMP2, LIT 0xFF (skipped), BRK
        let mut tape = make_tape(&[0xa0, 0x00, 0x06, 0x2c, 0x80, 0xFF, 0x00], 16);
        let steps = Uxn::execute(&mut tape, 256);
        assert_eq!(steps, 3); // LIT2, JMP2, BRK
    }

    #[test]
    fn jcn_taken() {
        // JCN2 pops condition (top), then address (below). Jumps if cond != 0.
        // Push address first, then condition: LIT2 0x0008, LIT 0x01, JCN2
        let mut tape = make_tape(&[0xa0, 0x00, 0x08, 0x80, 0x01, 0x2d, 0x80, 0xFF, 0x00], 16);
        let steps = Uxn::execute(&mut tape, 256);
        assert_eq!(steps, 4); // LIT2, LIT, JCN2, BRK
    }

    #[test]
    fn jcn_not_taken() {
        // Same layout but condition = 0 → falls through.
        let mut tape = make_tape(&[0xa0, 0x00, 0x08, 0x80, 0x00, 0x2d, 0x80, 0xFF, 0x00], 16);
        let steps = Uxn::execute(&mut tape, 256);
        // LIT2, LIT, JCN2 (not taken — only pops condition), LIT, BRK = 5
        assert_eq!(steps, 5);
    }

    #[test]
    fn dup_sta() {
        // LIT 0xBB, DUP → stack: [BB, BB].
        // Push addr, STA; push addr, STA.
        // Use 32-byte tape so store addresses (0x10, 0x11) don't overlap program code.
        let mut tape = make_tape(
            &[
                0x80, 0xBB, // LIT 0xBB
                0x06, // DUP
                0xa0, 0x00, 0x10, // LIT2 0x0010
                0x15, // STA
                0xa0, 0x00, 0x11, // LIT2 0x0011
                0x15, // STA
                0x00, // BRK
            ],
            32,
        );
        Uxn::execute(&mut tape, 256);
        assert_eq!(tape[0x10], 0xBB);
        assert_eq!(tape[0x11], 0xBB);
    }

    #[test]
    fn modular_addressing() {
        // On a 16-byte tape, address 0x0010 wraps to 0.
        // LIT 0xEE (value), LIT2 0x0010 (address), STA, BRK
        let mut tape = make_tape(&[0x80, 0xEE, 0xa0, 0x00, 0x10, 0x15, 0x00], 16);
        Uxn::execute(&mut tape, 256);
        assert_eq!(tape[0], 0xEE);
    }

    #[test]
    fn step_limit() {
        // Infinite loop: LIT2 0x0000, JMP2 → jumps back to 0 forever.
        let mut tape = make_tape(&[0xa0, 0x00, 0x00, 0x2c], 16);
        let steps = Uxn::execute(&mut tape, 100);
        assert_eq!(steps, 100);
    }

    #[test]
    fn empty_tape() {
        let mut tape: Vec<u8> = vec![];
        let steps = Uxn::execute(&mut tape, 256);
        assert_eq!(steps, 0);
    }

    #[test]
    fn disassemble() {
        let tape = vec![0x80, 0x42, 0x00]; // LIT 0x42; BRK
        let disasm = Uxn::disassemble(&tape);
        assert!(!disasm.is_empty());
        assert!(disasm.contains("LIT"));
        assert!(disasm.contains("BRK"));
    }

    #[test]
    fn div_by_zero() {
        // Uxn defines div-by-zero as producing 0.
        let mut tape = make_tape(&[0x80, 0x05, 0x80, 0x00, 0x1b, 0x00], 16);
        let steps = Uxn::execute(&mut tape, 256);
        assert_eq!(steps, 4);
    }

    #[test]
    fn stack_overflow_wraps() {
        // Push many values — stack pointer wraps at 256, should not panic.
        let mut tape = vec![0u8; 256];
        for i in 0..128 {
            tape[i * 2] = 0x80; // LIT
            tape[i * 2 + 1] = 0xFF; // value
        }
        let steps = Uxn::execute(&mut tape, 256);
        assert!(steps <= 256);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn never_panic(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let mut tape = tape_data;
            let steps = Uxn::execute(&mut tape, 256);
            prop_assert!(steps <= 256);
        }

        #[test]
        fn respect_step_limit(
            tape_data in prop::collection::vec(any::<u8>(), 1..256),
            limit in 1usize..500
        ) {
            let mut tape = tape_data;
            let steps = Uxn::execute(&mut tape, limit);
            prop_assert!(steps <= limit);
        }

        #[test]
        fn output_tape_same_length(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let original_len = tape_data.len();
            let mut tape = tape_data;
            Uxn::execute(&mut tape, 256);
            prop_assert_eq!(tape.len(), original_len);
        }
    }
}
