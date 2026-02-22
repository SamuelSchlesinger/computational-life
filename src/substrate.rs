/// A computational substrate that defines how programs are executed on a tape.
///
/// Each instruction set (BFF, Forth, Z80, etc.) implements this trait.
/// The simulation engine is generic over `Substrate`, so adding a new
/// instruction set requires only implementing this trait.
pub trait Substrate {
    /// Execute the program encoded in `tape`, starting from the beginning.
    ///
    /// The tape is modified in-place during execution (programs are
    /// self-modifying). Execution stops when the program terminates
    /// naturally or when `step_limit` steps have been consumed.
    ///
    /// Returns the number of steps actually executed.
    fn execute(tape: &mut [u8], step_limit: usize) -> usize;

    /// Returns true if the byte is a meaningful instruction in this substrate
    /// (as opposed to a no-op). Used for visualization.
    fn is_instruction(byte: u8) -> bool;

    /// Pretty-print a disassembly of the given tape for human inspection.
    fn disassemble(tape: &[u8]) -> String;
}
