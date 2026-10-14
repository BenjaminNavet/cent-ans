//! Oracle 3: an order the queries declared possible must not be refused.

use sim_campaign::{DiplomacyError, Order, OrderError};

use super::{Finding, OracleKind, Origin};

/// A finding when `result` of `submit_order(order)` is an `Err`; `None` when
/// the order was accepted. Only call it for orders the bot built from
/// candidates that the preview / prerequisite queries declared possible.
pub fn check(origin: &Origin, order: &Order, result: &Result<(), OrderError>) -> Option<Finding> {
    let error = result.as_ref().err()?;
    // A counterpart saying no to a proposal is a game outcome, not a bug: the
    // queries only promise the proposal is well-formed (`check_treaty`).
    if matches!(error, OrderError::Diplomacy(DiplomacyError::Refused(_))) {
        return None;
    }
    Some(origin.blocking(
        OracleKind::RefusedLegalOrder,
        format!("Ordre déclaré possible mais refusé : {error} ; ordre : {order:?}"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oracles::test_origin;

    #[test]
    fn accepted_order_is_silent_refused_one_is_reported() {
        let id = || sim_campaign::ArmyId::from_index(1);
        let order = Order::Attack {
            army: id(),
            target_army: id(),
        };
        assert!(check(&test_origin(), &order, &Ok(())).is_none());
        let err = OrderError::UnknownArmy(id());
        let finding = check(&test_origin(), &order, &Err(err)).expect("finding");
        assert_eq!(finding.oracle, OracleKind::RefusedLegalOrder);
        assert!(finding.message.contains("Attack"));
        let no = OrderError::Diplomacy(DiplomacyError::Refused("X refuse".into()));
        assert!(check(&test_origin(), &order, &Err(no)).is_none());
    }
}
