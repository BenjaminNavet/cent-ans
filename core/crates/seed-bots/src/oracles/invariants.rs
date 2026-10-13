//! Oracle 2: rule invariants (`sim_campaign::invariants`).

use sim_campaign::invariants::{check_invariants, Violation};

use super::{Finding, OracleContext, OracleKind, Origin};

/// One blocking finding per broken invariant of `ctx.state`.
pub fn check(ctx: &OracleContext<'_>, origin: &Origin) -> Vec<Finding> {
    from_violations(check_invariants(ctx.state, ctx.data), origin)
}

/// Converts violations into findings.
pub fn from_violations(violations: Vec<Violation>, origin: &Origin) -> Vec<Finding> {
    violations
        .into_iter()
        .map(|v| {
            origin.blocking(
                OracleKind::Invariant,
                format!("Invariant « {} » violé : {}", v.invariant, v.detail),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use data_model::{FactionId, GameData};
    use sim_campaign::{Army, ArmyPosition, CampaignState};

    use super::*;
    use crate::oracles::test_origin;

    #[test]
    fn sound_state_gives_nothing_and_corrupt_state_is_flagged() {
        let data = GameData::default();
        let faction = FactionId::new("fac_france").unwrap();
        let mut state = CampaignState::new(1);
        let origin = test_origin();
        fn ctx<'a>(
            state: &'a CampaignState,
            data: &'a GameData,
            faction: &'a FactionId,
        ) -> OracleContext<'a> {
            OracleContext {
                state,
                data,
                faction,
                turn: 3,
            }
        }
        assert!(check(&ctx(&state, &data, &faction), &origin).is_empty());

        let army = Army::new(
            FactionId::new("fac_nowhere").unwrap(),
            ArmyPosition::field([1.0, 2.0]),
            Vec::new(),
        );
        state
            .armies
            .insert(sim_campaign::ArmyId::from_index(1), army);
        let findings = check(&ctx(&state, &data, &faction), &origin);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].oracle, OracleKind::Invariant);
        assert!(findings[0].message.contains("army_refs"));
    }
}
