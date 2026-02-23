use crate::substrate::Substrate;

// The module `crate::mos6502` shadows the external crate name, so we use
// leading `::` to refer to the external crate throughout this file.
use ::mos6502::Variant;
use ::mos6502::cpu::CPU;
use ::mos6502::instruction::{AddressingMode, Nmos6502};
use ::mos6502::memory::Bus;

/// Wraps a byte-slice tape as a 6502 memory bus with modular addressing.
///
/// All addresses wrap modulo the tape length, giving programs a toroidal
/// memory space. The 6502's hardwired stack page (0x0100-0x01FF) and
/// interrupt vectors (0xFFFA-0xFFFF) simply map into the tape via modular
/// arithmetic, creating natural self-modification.
struct TapeBus<'a> {
    tape: &'a mut [u8],
}

impl Bus for TapeBus<'_> {
    fn get_byte(&mut self, address: u16) -> u8 {
        if self.tape.is_empty() {
            return 0;
        }
        self.tape[address as usize % self.tape.len()]
    }

    fn set_byte(&mut self, address: u16, value: u8) {
        if self.tape.is_empty() {
            return;
        }
        let idx = address as usize % self.tape.len();
        self.tape[idx] = value;
    }
}

/// The MOS 6502 CPU (NMOS variant including illegal opcodes).
///
/// The NMOS 6502 decodes almost every byte as an instruction, including
/// "illegal" opcodes like LAX, SAX, DCP, etc. Only 12 JAM opcodes
/// (0x02, 0x12, 0x22, 0x32, 0x42, 0x52, 0x62, 0x72, 0x92, 0xB2, 0xD2,
/// 0xF2) halt the CPU, creating a 4.7% chance per byte of a natural halt.
///
/// BRK (0x00) is NOT a halt — it's a software interrupt that pushes PC+2
/// and flags to the stack, then jumps to the IRQ vector (0xFFFE-0xFFFF).
/// With modular addressing on a random tape, this jumps to a random location.
pub struct Mos6502;

impl Substrate for Mos6502 {
    fn execute(tape: &mut [u8], step_limit: usize) -> usize {
        if tape.is_empty() {
            return 0;
        }
        let bus = TapeBus { tape };
        let mut cpu = CPU::new(bus, Nmos6502);
        // reset() reads the reset vector from 0xFFFC-0xFFFD (random bytes
        // on the tape). We override PC to 0 for consistency with other substrates.
        cpu.reset();
        cpu.registers.program_counter = 0;

        let mut steps = 0;
        while steps < step_limit {
            let ok = cpu.single_step();
            steps += 1;
            if !ok {
                break; // JAM or unrecognized opcode — halt
            }
        }
        steps
    }

    fn is_instruction(_byte: u8) -> bool {
        // NMOS 6502 decodes all 256 bytes (including illegal opcodes).
        true
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
            let (mnemonic, mode, size) = match Nmos6502::decode(opcode) {
                Some((instr, am)) => {
                    let sz = 1 + extra_bytes(am);
                    (format!("{:?}", instr), Some(am), sz)
                }
                None => ("???".to_string(), None, 1),
            };

            // Hex dump of instruction bytes
            let mut hex = String::new();
            for i in 0..size {
                let idx = (addr + i) % tape.len();
                if i > 0 {
                    hex.push(' ');
                }
                write!(hex, "{:02X}", tape[idx]).unwrap();
            }

            // Format operand based on addressing mode
            let operand = if size == 2 {
                let val = tape[(addr + 1) % tape.len()];
                format_operand_byte(mode, val)
            } else if size == 3 {
                let lo = tape[(addr + 1) % tape.len()];
                let hi = tape[(addr + 2) % tape.len()];
                let word = u16::from_le_bytes([lo, hi]);
                format_operand_word(mode, word)
            } else {
                String::new()
            };

            let _ = writeln!(out, "{addr:04X}: {hex:<12} {mnemonic}{operand}");
            addr += size;
        }
        out
    }
}

/// Return the number of operand bytes for a given addressing mode.
fn extra_bytes(mode: AddressingMode) -> usize {
    match mode {
        AddressingMode::Accumulator | AddressingMode::Implied => 0,
        AddressingMode::Immediate
        | AddressingMode::ZeroPage
        | AddressingMode::ZeroPageX
        | AddressingMode::ZeroPageY
        | AddressingMode::Relative
        | AddressingMode::IndexedIndirectX
        | AddressingMode::IndirectIndexedY => 1,
        AddressingMode::Absolute
        | AddressingMode::AbsoluteX
        | AddressingMode::AbsoluteY
        | AddressingMode::Indirect
        | AddressingMode::BuggyIndirect => 2,
        // Catch-all for any modes we don't explicitly list (e.g. CMOS-only).
        _ => 0,
    }
}

/// Format a single-byte operand in standard 6502 notation.
fn format_operand_byte(mode: Option<AddressingMode>, val: u8) -> String {
    match mode {
        Some(AddressingMode::Immediate) => format!(" #${val:02X}"),
        Some(AddressingMode::ZeroPage) => format!(" ${val:02X}"),
        Some(AddressingMode::ZeroPageX) => format!(" ${val:02X},X"),
        Some(AddressingMode::ZeroPageY) => format!(" ${val:02X},Y"),
        Some(AddressingMode::Relative) => format!(" ${val:02X}"),
        Some(AddressingMode::IndexedIndirectX) => format!(" (${val:02X},X)"),
        Some(AddressingMode::IndirectIndexedY) => format!(" (${val:02X}),Y"),
        _ => format!(" ${val:02X}"),
    }
}

/// Format a two-byte operand in standard 6502 notation.
fn format_operand_word(mode: Option<AddressingMode>, word: u16) -> String {
    match mode {
        Some(AddressingMode::Absolute) => format!(" ${word:04X}"),
        Some(AddressingMode::AbsoluteX) => format!(" ${word:04X},X"),
        Some(AddressingMode::AbsoluteY) => format!(" ${word:04X},Y"),
        Some(AddressingMode::Indirect) | Some(AddressingMode::BuggyIndirect) => {
            format!(" (${word:04X})")
        }
        _ => format!(" ${word:04X}"),
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
    fn nop() {
        // NOP = 0xEA
        let mut tape = make_tape(&[0xEA], 16);
        let steps = Mos6502::execute(&mut tape, 256);
        assert!(steps > 0);
    }

    #[test]
    fn jam() {
        // JAM = 0x02. CPU halts after executing it.
        // single_step() returns true on the JAM step, false on the next.
        let mut tape = make_tape(&[0x02], 16);
        let steps = Mos6502::execute(&mut tape, 256);
        // JAM executes (step 1, returns true), then next call returns false (step 2).
        assert_eq!(steps, 2);
    }

    #[test]
    fn lda_imm() {
        // LDA #$42 = 0xA9 0x42
        let mut tape = make_tape(&[0xA9, 0x42, 0x02], 16);
        Mos6502::execute(&mut tape, 256);
        // Just verify it runs without panic.
    }

    #[test]
    fn sta_abs() {
        // LDA #$AB = 0xA9 0xAB
        // STA $0080 = 0x8D 0x80 0x00
        // JAM = 0x02
        let mut tape = make_tape(&[0xA9, 0xAB, 0x8D, 0x80, 0x00, 0x02], 256);
        Mos6502::execute(&mut tape, 256);
        assert_eq!(tape[0x80], 0xAB);
    }

    #[test]
    fn jmp_abs() {
        // JMP $0005 = 0x4C 0x05 0x00
        // JAM (skipped) = 0x02
        // NOP = 0xEA
        // LDA #$FF = 0xA9 0xFF
        // STA $0080 = 0x8D 0x80 0x00
        // JAM = 0x02
        let mut tape = make_tape(
            &[
                0x4C, 0x05, 0x00, 0x02, 0xEA, 0xA9, 0xFF, 0x8D, 0x80, 0x00, 0x02,
            ],
            256,
        );
        Mos6502::execute(&mut tape, 256);
        assert_eq!(tape[0x80], 0xFF);
    }

    #[test]
    fn lda_sta_roundtrip() {
        // LDA #$55, STA $0080, LDA #$00, LDA $0080 → A should be $55
        // We verify by storing A again to $0081.
        let mut tape = make_tape(
            &[
                0xA9, 0x55, // LDA #$55
                0x8D, 0x80, 0x00, // STA $0080
                0xA9, 0x00, // LDA #$00 (clear A)
                0xAD, 0x80, 0x00, // LDA $0080 (reload)
                0x8D, 0x81, 0x00, // STA $0081
                0x02, // JAM
            ],
            256,
        );
        Mos6502::execute(&mut tape, 256);
        assert_eq!(tape[0x80], 0x55);
        assert_eq!(tape[0x81], 0x55);
    }

    #[test]
    fn pha_pla_roundtrip() {
        // LDA #$AA, PHA, LDA #$00, PLA → A should be $AA.
        // Verify by storing to $0080.
        // Stack is at 0x0100-0x01FF, which wraps on a 256-byte tape to 0x00-0xFF.
        let mut tape = make_tape(
            &[
                0xA9, 0xAA, // LDA #$AA
                0x48, // PHA
                0xA9, 0x00, // LDA #$00
                0x68, // PLA
                0x8D, 0x80, 0x00, // STA $0080
                0x02, // JAM
            ],
            256,
        );
        Mos6502::execute(&mut tape, 256);
        assert_eq!(tape[0x80], 0xAA);
    }

    #[test]
    fn branch() {
        // CLC (clear carry), BCC +$02 (branch if carry clear: skip 2 bytes)
        // JAM (skipped), JAM (skipped), LDA #$CC, STA $0080, JAM
        let mut tape = make_tape(
            &[
                0x18, // CLC
                0x90, 0x02, // BCC +2
                0x02, // JAM (skipped)
                0x02, // JAM (skipped)
                0xA9, 0xCC, // LDA #$CC
                0x8D, 0x80, 0x00, // STA $0080
                0x02, // JAM
            ],
            256,
        );
        Mos6502::execute(&mut tape, 256);
        assert_eq!(tape[0x80], 0xCC);
    }

    #[test]
    fn modular_addressing() {
        // On a 32-byte tape, address 0x0020 wraps to 0.
        // LDA #$EE, STA $0020, JAM
        let mut tape = make_tape(&[0xA9, 0xEE, 0x8D, 0x20, 0x00, 0x02], 32);
        Mos6502::execute(&mut tape, 256);
        assert_eq!(tape[0], 0xEE);
    }

    #[test]
    fn step_limit() {
        // JMP $0000 — infinite loop
        let mut tape = make_tape(&[0x4C, 0x00, 0x00], 16);
        let steps = Mos6502::execute(&mut tape, 100);
        assert_eq!(steps, 100);
    }

    #[test]
    fn empty_tape() {
        let mut tape: Vec<u8> = vec![];
        let steps = Mos6502::execute(&mut tape, 256);
        assert_eq!(steps, 0);
    }

    #[test]
    fn disassemble() {
        let tape = vec![0xA9, 0x42, 0x02]; // LDA #$42; JAM
        let disasm = Mos6502::disassemble(&tape);
        assert!(!disasm.is_empty());
        assert!(disasm.contains("LDA"));
    }

    #[test]
    fn all_jam_opcodes() {
        // All 12 JAM opcodes should halt the CPU.
        let jam_opcodes: [u8; 12] = [
            0x02, 0x12, 0x22, 0x32, 0x42, 0x52, 0x62, 0x72, 0x92, 0xB2, 0xD2, 0xF2,
        ];
        for &opcode in &jam_opcodes {
            let mut tape = make_tape(&[opcode], 16);
            let steps = Mos6502::execute(&mut tape, 256);
            assert!(steps <= 3, "JAM opcode {opcode:#04X} ran for {steps} steps");
        }
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
            let steps = Mos6502::execute(&mut tape, 256);
            prop_assert!(steps <= 256);
        }

        #[test]
        fn respect_step_limit(
            tape_data in prop::collection::vec(any::<u8>(), 1..256),
            limit in 1usize..500
        ) {
            let mut tape = tape_data;
            let steps = Mos6502::execute(&mut tape, limit);
            prop_assert!(steps <= limit);
        }

        #[test]
        fn output_tape_same_length(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let original_len = tape_data.len();
            let mut tape = tape_data;
            Mos6502::execute(&mut tape, 256);
            prop_assert_eq!(tape.len(), original_len);
        }
    }
}
