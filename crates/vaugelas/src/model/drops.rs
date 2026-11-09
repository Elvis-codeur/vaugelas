//! The data path's drop report (fr-check-infer `frc_prep::drop_report::Drops`) counts what training-data builds drop,
//! cut or change. At inference nothing reads those counters (`record_pf` discards them), so the generator and the
//! record builder report into this sink, which keeps their code identical to the parity-tested port and costs nothing.
#[derive(Clone, Copy, Debug, Default)]
pub struct Drops;

impl Drops {
    pub fn new() -> Self { Drops }
    #[inline]
    pub fn add(&mut self, _name: &str, _n: i64) {}
    #[inline]
    pub fn count(&mut self, _name: &str) {}
    /// the example is never built
    #[inline]
    pub fn count_ex(&mut self, _name: &str, _ex: impl FnOnce() -> String) {}
    #[inline]
    pub fn warn(&mut self, _name: &str, _msg: String) {}
}
