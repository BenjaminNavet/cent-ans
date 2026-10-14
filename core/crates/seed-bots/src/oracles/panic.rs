//! Oracle 1a: panics and slow turns.

use std::any::Any;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::Instant;

use super::{Finding, OracleKind, Origin};

/// Runs `step` under `catch_unwind`. Returns its result (`None` after a
/// panic) and the findings: a `Panic` with the panic message, or a
/// `SlowTurn` when the step lasted more than `slow_turn_ms`.
pub fn guarded<T>(
    origin: &Origin,
    slow_turn_ms: u64,
    step: impl FnOnce() -> T,
) -> (Option<T>, Vec<Finding>) {
    let started = Instant::now();
    let result = catch_unwind(AssertUnwindSafe(step));
    let elapsed_ms = started.elapsed().as_millis() as u64;
    let mut findings = Vec::new();
    let value = match result {
        Ok(value) => Some(value),
        Err(payload) => {
            findings.push(origin.blocking(
                OracleKind::Panic,
                format!(
                    "Panique pendant le tour : {}",
                    panic_message(payload.as_ref())
                ),
            ));
            None
        }
    };
    if elapsed_ms > slow_turn_ms {
        findings.push(origin.blocking(
            OracleKind::SlowTurn,
            format!("Tour trop lent : {elapsed_ms} ms (limite {slow_turn_ms} ms)."),
        ));
    }
    (value, findings)
}

/// Text of a panic payload (`&str` or `String`, else a placeholder).
pub fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_owned()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "charge utile de panique inconnue".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oracles::test_origin;

    #[test]
    fn quiet_step_gives_its_value() {
        let (value, findings) = guarded(&test_origin(), 1000, || 7);
        assert_eq!(value, Some(7));
        assert!(findings.is_empty());
    }

    #[test]
    fn panic_is_caught_with_its_message() {
        let (value, findings) = guarded(&test_origin(), 1000, || -> u8 { panic!("boom {}", 1) });
        assert_eq!(value, None);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].oracle, OracleKind::Panic);
        assert!(findings[0].message.contains("boom 1"));
        assert_eq!(findings[0].order_log.len(), 3);
    }

    #[test]
    fn slow_step_is_flagged() {
        let (value, findings) = guarded(&test_origin(), 5, || {
            std::thread::sleep(std::time::Duration::from_millis(30));
            1
        });
        assert_eq!(value, Some(1));
        assert_eq!(findings[0].oracle, OracleKind::SlowTurn);
    }
}
