//! Oracle 1b: save -> load -> save must be identical.

use sim_campaign::CampaignState;

use super::{Finding, OracleKind, Origin};

/// Saves `state`, loads the result and saves again; a finding when the load
/// fails or the two saves differ (the first difference is located).
pub fn check(state: &CampaignState, origin: &Origin) -> Option<Finding> {
    let first = state.save_json();
    let loaded = match CampaignState::load_json(&first) {
        Ok(loaded) => loaded,
        Err(error) => {
            return Some(origin.blocking(
                OracleKind::SaveRoundtrip,
                format!("La sauvegarde ne se recharge pas : {error}."),
            ));
        }
    };
    let second = loaded.save_json();
    if first == second {
        return None;
    }
    Some(origin.blocking(
        OracleKind::SaveRoundtrip,
        format!(
            "Sauvegarde, chargement, sauvegarde : résultat différent, premier écart {}.",
            first_difference(&first, &second)
        ),
    ))
}

/// Short description of where two JSON documents first differ.
pub fn first_difference(a: &str, b: &str) -> String {
    match (
        serde_json::from_str::<serde_json::Value>(a),
        serde_json::from_str::<serde_json::Value>(b),
    ) {
        (Ok(a), Ok(b)) => {
            let mut path = String::new();
            diff(&a, &b, &mut path).unwrap_or_else(|| "(aucun)".to_owned())
        }
        _ => "(JSON illisible)".to_owned(),
    }
}

fn diff(a: &serde_json::Value, b: &serde_json::Value, path: &mut String) -> Option<String> {
    use serde_json::Value;
    let here = |path: &str| {
        if path.is_empty() {
            "/".to_owned()
        } else {
            path.to_owned()
        }
    };
    let clip = |v: &Value| {
        let text = v.to_string();
        if text.chars().count() > 40 {
            format!("{}…", text.chars().take(40).collect::<String>())
        } else {
            text
        }
    };
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for key in x.keys().chain(y.keys()) {
                let len = path.len();
                path.push('/');
                path.push_str(key);
                let found = match (x.get(key), y.get(key)) {
                    (Some(l), Some(r)) => diff(l, r, path),
                    (l, r) => Some(format!(
                        "{} ({} -> {})",
                        here(path),
                        l.map_or("absent".to_owned(), clip),
                        r.map_or("absent".to_owned(), clip)
                    )),
                };
                path.truncate(len);
                if found.is_some() {
                    return found;
                }
            }
            None
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                return Some(format!(
                    "{} (longueurs {} et {})",
                    here(path),
                    x.len(),
                    y.len()
                ));
            }
            for (i, (l, r)) in x.iter().zip(y).enumerate() {
                let len = path.len();
                path.push_str(&format!("/{i}"));
                let found = diff(l, r, path);
                path.truncate(len);
                if found.is_some() {
                    return found;
                }
            }
            None
        }
        _ if a == b => None,
        _ => Some(format!("{} ({} -> {})", here(path), clip(a), clip(b))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oracles::test_origin;

    #[test]
    fn empty_state_round_trips() {
        let state = CampaignState::new(5);
        assert!(check(&state, &test_origin()).is_none());
    }

    #[test]
    fn difference_is_located() {
        let text = first_difference(r#"{"a":{"b":[1,2,3]}}"#, r#"{"a":{"b":[1,9,3]}}"#);
        assert!(text.starts_with("/a/b/1"), "{text}");
        assert!(first_difference("{}", "{}").contains("aucun"));
    }

    #[test]
    fn unreadable_save_is_a_finding() {
        // load_json refuses garbage: exercise the same message path through the origin.
        let finding = CampaignState::load_json("pas du json")
            .err()
            .map(|e| test_origin().blocking(OracleKind::SaveRoundtrip, e.to_string()));
        assert!(finding.is_some());
    }
}
