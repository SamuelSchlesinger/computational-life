use std::cell::{Cell, RefCell};

use crate::substrate::Substrate;
use iz80::{Cpu, Machine, Reg16};

/// Wrapper that presents a byte-slice tape as Z80/8080 memory with modular addressing.
///
/// All addresses wrap modulo the tape length, giving programs a toroidal
/// memory space identical to the other substrates. I/O ports are unused.
///
/// The `reads_left` counter prevents the iz80 decoder from looping forever
/// on DD/FD prefix chains — once exhausted, `peek` returns HALT (0x76) to
/// break out of the prefix-consuming `while` loop in `DecoderZ80::decode`.
struct TapeMachine<'a> {
    tape: &'a mut [u8],
    /// Reads remaining before we force a HALT. Uses `Cell` because
    /// `Machine::peek` takes `&self`.
    reads_left: Cell<usize>,
}

impl Machine for TapeMachine<'_> {
    fn peek(&self, address: u16) -> u8 {
        let left = self.reads_left.get();
        if left == 0 {
            return 0x76; // HALT — breaks any internal decoder loops
        }
        self.reads_left.set(left - 1);
        if self.tape.is_empty() {
            return 0;
        }
        self.tape[address as usize % self.tape.len()]
    }

    fn poke(&mut self, address: u16, value: u8) {
        if self.tape.is_empty() {
            return;
        }
        let idx = address as usize % self.tape.len();
        self.tape[idx] = value;
    }

    fn port_in(&mut self, _address: u16) -> u8 {
        0
    }

    fn port_out(&mut self, _address: u16, _value: u8) {}
}

/// Reset a CPU to a clean initial state for a new execution.
///
/// If the CPU was halted by a previous run, `signal_reset` + one
/// `execute_instruction` call clears the halt flag and resets PC/SP/AF.
/// We then zero the remaining registers that `reset()` leaves undefined.
fn reset_cpu(cpu: &mut Cpu, tape: &mut [u8]) {
    if cpu.is_halted() {
        cpu.signal_reset();
        // Process the pending reset (clears halted, resets PC/SP/AF).
        let max_reads = tape.len().max(16);
        let mut machine = TapeMachine {
            tape,
            reads_left: Cell::new(max_reads),
        };
        cpu.execute_instruction(&mut machine);
    } else {
        let regs = cpu.registers();
        regs.set_pc(0);
        regs.set16(Reg16::AF, 0xFFFF);
        regs.set16(Reg16::SP, 0xFFFF);
    }
    // Zero registers that reset() leaves undefined.
    let regs = cpu.registers();
    regs.set16(Reg16::BC, 0);
    regs.set16(Reg16::DE, 0);
    regs.set16(Reg16::HL, 0);
    regs.set16(Reg16::IX, 0);
    regs.set16(Reg16::IY, 0);
}

/// Execute instructions on the tape using the given CPU, returning the step count.
fn execute_cpu(cpu: &mut Cpu, tape: &mut [u8], step_limit: usize) -> usize {
    if tape.is_empty() {
        return 0;
    }
    // A single Z80 instruction reads at most ~4 bytes for fetch + a few
    // data bytes. We allow tape.len() reads per instruction to handle any
    // realistic instruction, while guaranteeing that a DD/FD prefix chain
    // wrapping around the modular tape will terminate.
    let max_reads = tape.len().max(16);
    reset_cpu(cpu, tape);
    let mut machine = TapeMachine {
        tape,
        reads_left: Cell::new(max_reads),
    };
    let mut steps = 0;
    while steps < step_limit && !cpu.is_halted() {
        machine.reads_left.set(max_reads);
        cpu.execute_instruction(&mut machine);
        steps += 1;
    }
    steps
}

thread_local! {
    static Z80_CPU: RefCell<Cpu> = RefCell::new(Cpu::new());
    static I8080_CPU: RefCell<Cpu> = RefCell::new(Cpu::new_8080());
}

/// Disassemble the tape contents using the given CPU's instruction decoder.
fn disassemble_cpu(cpu: &mut Cpu, tape: &[u8]) -> String {
    use std::fmt::Write;
    if tape.is_empty() {
        return String::new();
    }
    // We need a mutable copy for the Machine trait, but disassembly won't modify it.
    let mut buf = tape.to_vec();
    let max_reads = buf.len().max(16);
    let mut machine = TapeMachine {
        tape: &mut buf,
        reads_left: Cell::new(max_reads),
    };
    let mut out = String::new();
    cpu.registers().set_pc(0);
    let mut addr = 0u16;
    while (addr as usize) < tape.len() {
        let line = cpu.disasm_instruction(&mut machine);
        let new_pc = cpu.registers().pc();
        let byte_count = if new_pc > addr {
            (new_pc - addr) as usize
        } else {
            // Wrapped or single-byte at end
            1
        };
        let mut hex = String::new();
        for i in 0..byte_count {
            let idx = (addr as usize + i) % tape.len();
            if i > 0 {
                hex.push(' ');
            }
            write!(hex, "{:02X}", tape[idx]).unwrap();
        }
        let _ = writeln!(out, "{addr:04X}: {hex:<12} {line}");
        addr = new_pc;
        if addr == 0 && byte_count > 0 {
            break; // Wrapped around
        }
    }
    out
}

/// The Zilog Z80 instruction set (Section 3.3 of the paper).
///
/// 16-byte programs on modular memory with a 256-step limit. The paper
/// reports stack-based replicators (PUSH HL) emerging first, later overtaken
/// by LDIR/LDDR block-copy replicators.
pub struct Z80;

impl Substrate for Z80 {
    fn execute(tape: &mut [u8], step_limit: usize) -> usize {
        Z80_CPU.with_borrow_mut(|cpu| execute_cpu(cpu, tape, step_limit))
    }

    fn is_instruction(_byte: u8) -> bool {
        // Both Z80 opcode tables are fully populated — every byte decodes
        // to some instruction (including prefix bytes like CB, DD, ED, FD).
        true
    }

    fn disassemble(tape: &[u8]) -> String {
        let mut cpu = Cpu::new();
        disassemble_cpu(&mut cpu, tape)
    }
}

/// The Intel 8080 instruction set (Section 3.3 of the paper).
///
/// The paper reports 2-byte non-looping replicators (e.g. `01 c5` = LXI BC /
/// PUSH BC) that replicate efficiently but never develop looping variants.
pub struct I8080;

impl Substrate for I8080 {
    fn execute(tape: &mut [u8], step_limit: usize) -> usize {
        I8080_CPU.with_borrow_mut(|cpu| execute_cpu(cpu, tape, step_limit))
    }

    fn is_instruction(_byte: u8) -> bool {
        true
    }

    fn disassemble(tape: &[u8]) -> String {
        let mut cpu = Cpu::new_8080();
        disassemble_cpu(&mut cpu, tape)
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

    // ─── Z80 tests ──────────────────────────────────────────────────────

    #[test]
    fn z80_nop() {
        let mut tape = make_tape(&[0x00], 16);
        let steps = Z80::execute(&mut tape, 256);
        assert!(steps > 0);
    }

    #[test]
    fn z80_halt() {
        // HALT = 0x76. CPU should stop after executing it.
        let mut tape = make_tape(&[0x76], 16);
        let steps = Z80::execute(&mut tape, 256);
        assert_eq!(steps, 1);
    }

    #[test]
    fn z80_ld_a_immediate() {
        // LD A, 0x42 = 0x3E 0x42
        let mut tape = make_tape(&[0x3E, 0x42, 0x76], 16);
        Z80::execute(&mut tape, 256);
        // Verify via a store: LD (HL), A = 0x77 would store A at address in HL.
        // HL starts at 0, so it would store at address 0 — but that overwrites
        // our program. Instead just confirm it runs and halts.
    }

    #[test]
    fn z80_push_pop() {
        // LD HL, 0x1234 = 0x21 0x34 0x12
        // PUSH HL = 0xE5
        // POP BC = 0xC1
        // HALT = 0x76
        let mut tape = make_tape(&[0x21, 0x34, 0x12, 0xE5, 0xC1, 0x76], 256);
        let steps = Z80::execute(&mut tape, 256);
        assert!(steps <= 6);
    }

    #[test]
    fn z80_ld_store() {
        // LD A, 0xAB = 0x3E 0xAB
        // LD (0x0080), A = 0x32 0x80 0x00
        // HALT = 0x76
        let mut tape = make_tape(&[0x3E, 0xAB, 0x32, 0x80, 0x00, 0x76], 256);
        Z80::execute(&mut tape, 256);
        assert_eq!(tape[0x80], 0xAB);
    }

    #[test]
    fn z80_jp() {
        // JP 0x0004 = 0xC3 0x04 0x00
        // HALT (skipped) = 0x76
        // LD A, 0xFF = 0x3E 0xFF
        // LD (0x0080), A = 0x32 0x80 0x00
        // HALT = 0x76
        let mut tape = make_tape(
            &[0xC3, 0x04, 0x00, 0x76, 0x3E, 0xFF, 0x32, 0x80, 0x00, 0x76],
            256,
        );
        Z80::execute(&mut tape, 256);
        assert_eq!(tape[0x80], 0xFF);
    }

    #[test]
    fn z80_jr() {
        // JR +2 = 0x18 0x02 (jumps over next 2 bytes)
        // HALT (skipped) = 0x76
        // HALT (skipped) = 0x76
        // LD A, 0xCC = 0x3E 0xCC
        // LD (0x0080), A = 0x32 0x80 0x00
        // HALT = 0x76
        let mut tape = make_tape(
            &[0x18, 0x02, 0x76, 0x76, 0x3E, 0xCC, 0x32, 0x80, 0x00, 0x76],
            256,
        );
        Z80::execute(&mut tape, 256);
        assert_eq!(tape[0x80], 0xCC);
    }

    #[test]
    fn z80_ldir() {
        // Set up source data at 0x0080
        // LD HL, 0x0080 = 0x21 0x80 0x00 (source)
        // LD DE, 0x0090 = 0x11 0x90 0x00 (dest)
        // LD BC, 0x0004 = 0x01 0x04 0x00 (count)
        // LDIR = 0xED 0xB0
        // HALT = 0x76
        let mut tape = make_tape(
            &[
                0x21, 0x80, 0x00, // LD HL, 0x0080
                0x11, 0x90, 0x00, // LD DE, 0x0090
                0x01, 0x04, 0x00, // LD BC, 0x0004
                0xED, 0xB0, // LDIR
                0x76, // HALT
            ],
            256,
        );
        tape[0x80] = 0xAA;
        tape[0x81] = 0xBB;
        tape[0x82] = 0xCC;
        tape[0x83] = 0xDD;
        Z80::execute(&mut tape, 256);
        assert_eq!(tape[0x90], 0xAA);
        assert_eq!(tape[0x91], 0xBB);
        assert_eq!(tape[0x92], 0xCC);
        assert_eq!(tape[0x93], 0xDD);
    }

    #[test]
    fn z80_lddr() {
        // LDDR copies backwards: HL and DE decrement, BC counts down.
        // LD HL, 0x0083 = 0x21 0x83 0x00 (source end)
        // LD DE, 0x0093 = 0x11 0x93 0x00 (dest end)
        // LD BC, 0x0004 = 0x01 0x04 0x00 (count)
        // LDDR = 0xED 0xB8
        // HALT = 0x76
        let mut tape = make_tape(
            &[
                0x21, 0x83, 0x00, 0x11, 0x93, 0x00, 0x01, 0x04, 0x00, 0xED, 0xB8, 0x76,
            ],
            256,
        );
        tape[0x80] = 0x11;
        tape[0x81] = 0x22;
        tape[0x82] = 0x33;
        tape[0x83] = 0x44;
        Z80::execute(&mut tape, 256);
        assert_eq!(tape[0x90], 0x11);
        assert_eq!(tape[0x91], 0x22);
        assert_eq!(tape[0x92], 0x33);
        assert_eq!(tape[0x93], 0x44);
    }

    #[test]
    fn z80_modular_addressing() {
        // LD A, 0xEE; LD (addr), A — where addr wraps around a 32-byte tape.
        // Address 0x0020 wraps to 0 on a 32-byte tape.
        let mut tape = make_tape(&[0x3E, 0xEE, 0x32, 0x20, 0x00, 0x76], 32);
        Z80::execute(&mut tape, 256);
        // 0x0020 % 32 = 0, but tape[0] was 0x3E (our LD A instruction).
        // The store overwrites it.
        assert_eq!(tape[0], 0xEE);
    }

    #[test]
    fn z80_sp_wraps() {
        // On a 16-byte tape, SP starts at 0. PUSH decrements SP by 2,
        // wrapping into the upper tape region via modular addressing.
        // Verify by PUSH then POP round-trip: the value survives.
        // LD HL, 0x1234; PUSH HL; POP DE; HALT
        // Then check DE got the right value by storing E and D.
        //
        // Use a 256-byte tape so the store addresses don't collide with
        // the stack region.
        let mut tape = make_tape(
            &[
                0x21, 0x34, 0x12, // LD HL, 0x1234
                0xE5, // PUSH HL
                0xD1, // POP DE
                // Store E (low byte of DE) to 0x0080
                0x3E, 0x00, // LD A, 0 (placeholder)
                0x7B, // LD A, E
                0x32, 0x80, 0x00, // LD (0x0080), A
                // Store D (high byte of DE) to 0x0081
                0x7A, // LD A, D
                0x32, 0x81, 0x00, // LD (0x0081), A
                0x76, // HALT
            ],
            256,
        );
        Z80::execute(&mut tape, 256);
        assert_eq!(tape[0x80], 0x34); // low byte round-tripped
        assert_eq!(tape[0x81], 0x12); // high byte round-tripped
    }

    #[test]
    fn z80_step_limit() {
        // Create an infinite loop: JP 0x0000
        let mut tape = make_tape(&[0xC3, 0x00, 0x00], 16);
        let steps = Z80::execute(&mut tape, 100);
        assert_eq!(steps, 100);
    }

    #[test]
    fn z80_empty_tape() {
        let mut tape: Vec<u8> = vec![];
        let steps = Z80::execute(&mut tape, 256);
        assert_eq!(steps, 0);
    }

    #[test]
    fn z80_disassemble() {
        let tape = vec![0x3E, 0x42, 0x76]; // LD A, 0x42; HALT
        let disasm = Z80::disassemble(&tape);
        assert!(!disasm.is_empty());
        // Should contain something about LD and HALT
    }

    // ─── 8080 tests ─────────────────────────────────────────────────────

    #[test]
    fn i8080_nop() {
        let mut tape = make_tape(&[0x00], 16);
        let steps = I8080::execute(&mut tape, 256);
        assert!(steps > 0);
    }

    #[test]
    fn i8080_hlt() {
        // HLT = 0x76
        let mut tape = make_tape(&[0x76], 16);
        let steps = I8080::execute(&mut tape, 256);
        assert_eq!(steps, 1);
    }

    #[test]
    fn i8080_lxi_push() {
        // The paper's canonical 8080 replicator: 0x01 0xC5
        // LXI BC, ... loads BC. But 0x01 is LXI B which takes 2 more bytes.
        // In a 2-byte context the third byte would wrap. Let's test the
        // full LXI + PUSH pattern on a larger tape:
        // LXI B, 0x01C5 = 0x01 0xC5 0x01
        // PUSH B = 0xC5
        // HLT = 0x76
        let mut tape = make_tape(&[0x01, 0xC5, 0x01, 0xC5, 0x76], 256);
        let steps = I8080::execute(&mut tape, 256);
        assert!(steps <= 5);
    }

    #[test]
    fn i8080_mov() {
        // MVI A, 0x55 = 0x3E 0x55
        // MOV B, A = 0x47
        // HLT = 0x76
        let mut tape = make_tape(&[0x3E, 0x55, 0x47, 0x76], 256);
        I8080::execute(&mut tape, 256);
    }

    #[test]
    fn i8080_sta() {
        // MVI A, 0xBB = 0x3E 0xBB
        // STA 0x0080 = 0x32 0x80 0x00
        // HLT = 0x76
        let mut tape = make_tape(&[0x3E, 0xBB, 0x32, 0x80, 0x00, 0x76], 256);
        I8080::execute(&mut tape, 256);
        assert_eq!(tape[0x80], 0xBB);
    }

    #[test]
    fn i8080_step_limit() {
        // JMP 0x0000 = 0xC3 0x00 0x00 — infinite loop
        let mut tape = make_tape(&[0xC3, 0x00, 0x00], 16);
        let steps = I8080::execute(&mut tape, 100);
        assert_eq!(steps, 100);
    }

    #[test]
    fn i8080_empty_tape() {
        let mut tape: Vec<u8> = vec![];
        let steps = I8080::execute(&mut tape, 256);
        assert_eq!(steps, 0);
    }

    #[test]
    fn i8080_disassemble() {
        let tape = vec![0x3E, 0x42, 0x76]; // MVI A, 0x42; HLT
        let disasm = I8080::disassemble(&tape);
        assert!(!disasm.is_empty());
    }

    #[test]
    fn z80_dd_prefix_chain_terminates() {
        // All 0xDD bytes — the iz80 decoder has `while code == 0xdd || code == 0xfd`
        // which, with modular addressing, would loop forever without reads_left guard.
        let mut tape = vec![0xDD; 16];
        let steps = Z80::execute(&mut tape, 100);
        assert!(steps <= 100);
    }

    #[test]
    fn z80_fd_prefix_chain_terminates() {
        let mut tape = vec![0xFD; 16];
        let steps = Z80::execute(&mut tape, 100);
        assert!(steps <= 100);
    }

    #[test]
    fn z80_mixed_dd_fd_prefix_chain_terminates() {
        // Alternating DD/FD should also terminate.
        let mut tape: Vec<u8> = (0..16)
            .map(|i| if i % 2 == 0 { 0xDD } else { 0xFD })
            .collect();
        let steps = Z80::execute(&mut tape, 100);
        assert!(steps <= 100);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn z80_random_programs_never_panic(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let mut tape = tape_data;
            let steps = Z80::execute(&mut tape, 256);
            prop_assert!(steps <= 256);
        }

        #[test]
        fn z80_random_programs_respect_step_limit(
            tape_data in prop::collection::vec(any::<u8>(), 1..256),
            limit in 1usize..500
        ) {
            let mut tape = tape_data;
            let steps = Z80::execute(&mut tape, limit);
            prop_assert!(steps <= limit);
        }

        #[test]
        fn z80_output_tape_same_length(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let original_len = tape_data.len();
            let mut tape = tape_data;
            Z80::execute(&mut tape, 256);
            prop_assert_eq!(tape.len(), original_len);
        }

        #[test]
        fn i8080_random_programs_never_panic(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let mut tape = tape_data;
            let steps = I8080::execute(&mut tape, 256);
            prop_assert!(steps <= 256);
        }

        #[test]
        fn i8080_random_programs_respect_step_limit(
            tape_data in prop::collection::vec(any::<u8>(), 1..256),
            limit in 1usize..500
        ) {
            let mut tape = tape_data;
            let steps = I8080::execute(&mut tape, limit);
            prop_assert!(steps <= limit);
        }

        #[test]
        fn i8080_output_tape_same_length(tape_data in prop::collection::vec(any::<u8>(), 1..256)) {
            let original_len = tape_data.len();
            let mut tape = tape_data;
            I8080::execute(&mut tape, 256);
            prop_assert_eq!(tape.len(), original_len);
        }
    }
}
