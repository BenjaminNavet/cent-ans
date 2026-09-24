//! Historical side changes (G2, G4): Flanders ruined by the wool embargo
//! turns to England (Artevelde, 1338-1340); Burgundy abandons a beaten France
//! for the dominant invader (Troyes, 1420) or out of a blood feud
//! (Montereau, 1419); Edward III's in-laws and pensioners of the Low
//! Countries. Every threshold comes from `data/ai/alignment.json`; without
//! that file the AI makes no historical side change.

use std::collections::BTreeMap;

use data_model::{AiAlignment, FactionId, GameData, ProvinceId};
use sim_campaign::diplomacy::{
    claim_stakes, evaluate, rivals, war_ready, Proposal, RelationKind, AGGRESSION_REASON,
    AT_WAR_REASON, DESERTION_WAR_SCORE, GIFT_REASON, PERJURY_REASON,
};
use sim_campaign::movement::land_neighbors;
use sim_campaign::{CampaignState, Order};

/// Salt of the wool revolt roll.
const WOOL_SALT: u64 = 1;
/// Salt of the defection rolls (one per decade from this one).
const DEFECTION_SALT: u64 = 2;
/// Salt of the dynastic alliance and money fief roll.
const DYNASTIC_SALT: u64 = 7;
/// Turns per decade (one defection roll each).
const DECADE_TURNS: u32 = 40;

/// Pure per-campaign roll in 0..1000 for `faction` and `salt` (splitmix64
/// of the seed): the AI stays deterministic without touching the RNG.
pub fn campaign_roll(state: &CampaignState, faction: &FactionId, salt: u64) -> u64 {
    let mut x = state.seed ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for b in faction.as_str().bytes() {
        x = x.rotate_left(5) ^ u64::from(b);
    }
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    (x ^ (x >> 31)) % 1000
}

/// Planning slot of `faction` (same as `plan_diplomacy`'s).
fn slot(faction: &FactionId) -> u32 {
    faction
        .as_str()
        .bytes()
        .fold(0u32, |a, b| a.wrapping_add(u32::from(b)))
}

/// Provinces of `crown`'s realm: every province of the regions where it
/// owned `defection.realm_region_provinces` in 1337 (the kingdom of France
/// includes English Guyenne and Ponthieu).
pub fn realm_provinces<'a>(data: &'a GameData, crown: &FactionId) -> Vec<&'a ProvinceId> {
    let Some(rules) = data.ai_alignment.as_ref() else {
        return Vec::new();
    };
    let mut owned: BTreeMap<&str, usize> = BTreeMap::new();
    for p in data.provinces.values().filter(|p| &p.owner == crown) {
        *owned.entry(p.region.as_str()).or_default() += 1;
    }
    data.provinces
        .iter()
        .filter(|(_, p)| {
            owned
                .get(p.region.as_str())
                .is_some_and(|n| *n >= rules.defection.realm_region_provinces)
        })
        .map(|(id, _)| id)
        .collect()
}

/// Provinces of `crown`'s realm controlled by `holder`.
pub fn realm_held_by(
    state: &CampaignState,
    data: &GameData,
    crown: &FactionId,
    holder: &FactionId,
) -> usize {
    realm_provinces(data, crown)
        .into_iter()
        .filter(|id| state.controls_province(holder, id))
        .count()
}

/// Does `invader` dominate `crown`'s realm: it holds the 1337 capital or
/// `defection.dominance_realm_share` of its provinces, while `crown` has
/// lost `defection.crown_in_trouble_loss` of its own lands or is beaten.
pub fn dominates_realm(
    state: &CampaignState,
    data: &GameData,
    invader: &FactionId,
    crown: &FactionId,
) -> bool {
    let Some(rules) = data.ai_alignment.as_ref() else {
        return false;
    };
    let Some(capital) = data.factions.get(crown).map(|f| &f.capital) else {
        return false;
    };
    let seat = state.controls_province(invader, capital);
    let realm = realm_provinces(data, crown).len();
    let held = realm_held_by(state, data, crown, invader);
    let dominant = realm > 0 && held as f64 >= rules.defection.dominance_realm_share * realm as f64;
    // Trouble is measured on the crown's own lands of 1337.
    let own: Vec<_> = data
        .provinces
        .iter()
        .filter(|(_, p)| &p.owner == crown)
        .collect();
    let kept = own
        .iter()
        .filter(|(id, _)| state.controls_province(crown, id))
        .count();
    let in_trouble = (kept as f64)
        < (1.0 - rules.defection.crown_in_trouble_loss) * own.len() as f64
        || state.war_score(data, crown, invader) <= DESERTION_WAR_SCORE;
    (seat || dominant) && in_trouble
}

/// Can `faction` side with `invader` against `patron`: a living crown, not
/// rebels nor `patron`'s ally, with no claim on our lands (nor we on its).
fn eligible_invader(
    state: &CampaignState,
    faction: &FactionId,
    patron: &FactionId,
    invader: &FactionId,
) -> bool {
    invader != faction
        && invader != patron
        && invader.as_str() != "fac_rebels"
        && state.factions.get(invader).is_some_and(|f| f.alive)
        && !state.is_allied(invader, patron)
        && !claim_stakes(state, invader, faction).any()
        && !claim_stakes(state, faction, invader).any()
}

/// Sum of the grudges `faction` holds against `patron`: its running
/// negative opinion modifiers (a murder, a betrayal), whatever the bond.
/// The reputation every court holds against a perjurer or an aggressor is
/// no personal grudge.
pub fn grievance(state: &CampaignState, faction: &FactionId, patron: &FactionId) -> i32 {
    state.factions.get(faction).map_or(0, |f| {
        f.modifiers
            .iter()
            .filter(|m| &m.with == patron && m.expires_turn > state.turn && m.value < 0)
            .filter(|m| m.reason_fr != PERJURY_REASON && m.reason_fr != AGGRESSION_REASON)
            .map(|m| m.value)
            .sum()
    })
}

/// Attitude of `faction` towards `other`, leaving out the war between
/// them (a side change starts with a white peace).
fn attitude_beyond_war(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    other: &FactionId,
) -> i32 {
    let (_, reasons) = state.attitude(data, faction, other);
    let total: i32 = reasons
        .iter()
        .filter(|(label, _)| label != AT_WAR_REASON)
        .map(|(_, value)| value)
        .sum();
    total.clamp(-100, 100)
}

/// Blood feud (G4, Montereau): the patron `faction` holds a grievance of
/// `grievance.grudge` or worse against (its suzerain, an ally, or a crown
/// it already fights) and the enemy or pretender of that patron it would
/// side with (attitude of `grievance.min_attitude`, their war aside).
pub fn grievance_change(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Option<(FactionId, FactionId)> {
    let rules = &data.ai_alignment.as_ref()?.grievance;
    let me = state.factions.get(faction)?;
    let patrons = me
        .suzerain
        .iter()
        .chain(me.allies.iter())
        .chain(me.at_war_with.iter().filter(|e| e.as_str() != "fac_rebels"));
    for patron in patrons {
        if !state.factions.get(patron).is_some_and(|f| f.alive)
            || grievance(state, faction, patron) > rules.grudge
        {
            continue;
        }
        let feud_at_war = me.at_war_with.contains(patron);
        let best = state
            .factions
            .keys()
            .filter(|e| eligible_invader(state, faction, patron, e))
            .filter(|e| {
                let pretender = claim_stakes(state, e, patron).throne;
                let fights = state.is_at_war(e, patron);
                // A feud already at war only draws in the patron's
                // pretender (the Burgundians and the Lancastrian king),
                // who may not be fighting yet.
                if feud_at_war {
                    pretender
                } else {
                    pretender || fights
                }
            })
            .map(|e| (attitude_beyond_war(state, data, faction, e), e))
            .filter(|(towards, _)| *towards >= rules.min_attitude)
            .max_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(a.1)));
        if let Some((_, invader)) = best {
            return Some((patron.clone(), invader.clone()));
        }
    }
    None
}

/// The patron `faction` abandons and the crown it turns to, if any: a
/// vassal under the embargo of its lord's enemy (wool revolt), a vassal or
/// ally of a crown whose realm the enemy dominates (Troyes), or a prince
/// with a grievance against its patron (Montereau).
pub fn side_change(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Option<(FactionId, FactionId)> {
    let rules = data.ai_alignment.as_ref()?;
    let me = state.factions.get(faction)?;
    let alive = |f: &FactionId| state.factions.get(f).is_some_and(|s| s.alive);
    if let Some(lord) = me
        .suzerain
        .clone()
        .filter(|_| me.loyalty < rules.wool_revolt.loyalty)
        .filter(|_| campaign_roll(state, faction, WOOL_SALT) < rules.history_permille)
    {
        let embargoer = state
            .factions
            .iter()
            .find(|(id, f)| f.alive && f.embargoes.contains(faction) && state.is_at_war(&lord, id));
        if let Some((embargoer, _)) = embargoer {
            return Some((lord, embargoer.clone()));
        }
    }
    // A blood feud needs no roll: the chronicle already hesitated.
    if let Some(change) = grievance_change(state, data, faction) {
        return Some(change);
    }
    // One roll per decade: the fall of a crown is not written.
    let decade = u64::from(state.turn / DECADE_TURNS);
    if campaign_roll(state, faction, DEFECTION_SALT + decade) >= rules.history_permille {
        return None;
    }
    let patrons: Vec<FactionId> = match &me.suzerain {
        Some(lord) if me.loyalty < rules.defection.loyalty => vec![lord.clone()],
        Some(_) => Vec::new(),
        None => me.allies.iter().filter(|a| alive(a)).cloned().collect(),
    };
    patrons.into_iter().find_map(|patron| {
        let invader = state.factions[&patron]
            .at_war_with
            .iter()
            .filter(|e| eligible_invader(state, faction, &patron, e))
            .find(|e| dominates_realm(state, data, e, &patron))?
            .clone();
        Some((patron, invader))
    })
}

/// Orders of `faction` changing sides this turn (G2): white peace with the
/// new patron first, then independence war (vassal) or broken alliance,
/// and an alliance offer to the new patron.
pub fn plan_side_change(state: &CampaignState, data: &GameData, faction: &FactionId) -> Vec<Order> {
    if faction == &state.player_faction || !(state.turn + slot(faction)).is_multiple_of(4) {
        return Vec::new();
    }
    let Some((patron, invader)) = side_change(state, data, faction) else {
        return Vec::new();
    };
    let white = Proposal::Peace {
        provinces: Vec::new(),
        tribute: 0,
    };
    if state.is_at_war(faction, &invader) {
        let accepted = invader != state.player_faction
            && evaluate(state, data, faction, &invader, &white).accept;
        return if accepted {
            vec![Order::ProposePeace {
                target: invader,
                provinces: Vec::new(),
                tribute: 0,
            }]
        } else {
            Vec::new()
        };
    }
    let mut orders = Vec::new();
    let me = &state.factions[faction];
    if me.suzerain.as_ref() == Some(&patron) {
        if !war_ready(state, faction) {
            return orders;
        }
        orders.push(Order::DeclareWar {
            target: patron.clone(),
        });
    } else if state.is_allied(faction, &patron) {
        orders.push(Order::BreakAlliance {
            target: patron.clone(),
        });
    }
    if !state.is_allied(faction, &invader) {
        // The new camp takes no friend of its rivals: the old alliances
        // against it go first (Burgundy leaves the Scots to the dauphin).
        let theirs = rivals(state, &invader);
        orders.extend(
            me.allies
                .iter()
                .filter(|a| **a != patron && theirs.contains(*a))
                .filter(|a| state.relation(faction, a) == RelationKind::Alliance)
                .map(|a| Order::BreakAlliance { target: a.clone() }),
        );
        orders.push(Order::ProposeAlliance { target: invader });
    }
    orders
}

/// `true` when `a` and `b` control provinces adjacent on the map (land
/// borders as armies walk them). `CampaignState::are_neighbors` reads the
/// province files' `neighbors`, filled for six provinces only.
pub fn borders(state: &CampaignState, data: &GameData, a: &FactionId, b: &FactionId) -> bool {
    state.provinces.keys().any(|id| {
        state.controls_province(a, id)
            && land_neighbors(data, id)
                .iter()
                .any(|n| state.controls_province(b, n))
    })
}

/// Princes on the border of an enemy of `faction` (itself no vassal),
/// allied neither with it nor with that enemy, no enemy or pretender of
/// its allies, whose campaign roll lets history court them: each
/// with its attitude towards us and its best one towards our enemies.
fn courted_princes<'a>(
    state: &'a CampaignState,
    data: &GameData,
    rules: &AiAlignment,
    faction: &FactionId,
) -> Vec<(&'a FactionId, i32, i32)> {
    let Some(me) = state.factions.get(faction) else {
        return Vec::new();
    };
    // A vassal's diplomacy follows its lord's; a full coalition courts
    // no more (every prince on France's border is not England's in-law).
    let allies = me
        .allies
        .iter()
        .filter(|a| {
            state
                .factions
                .get(*a)
                .is_some_and(|f| f.suzerain.as_ref() != Some(faction))
        })
        .count();
    if me.suzerain.is_some() || allies >= rules.dynastic.max_allies {
        return Vec::new();
    }
    // Wars of succession only: princes are courted for a crown (Edward
    // III's claim on France), not for a border quarrel.
    let enemies: Vec<&FactionId> = me
        .at_war_with
        .iter()
        .filter(|e| e.as_str() != "fac_rebels")
        .filter(|e| {
            claim_stakes(state, faction, e).throne || claim_stakes(state, e, faction).throne
        })
        .collect();
    if enemies.is_empty() {
        return Vec::new();
    }
    // A great crown is no pensioner.
    let lesser = |id: &FactionId| {
        state.faction_power(id) <= rules.dynastic.max_power_ratio * state.faction_power(faction)
    };
    state
        .factions
        .iter()
        .filter(|(id, f)| {
            *id != faction && f.alive && f.suzerain.is_none() && id.as_str() != "fac_rebels"
        })
        .filter(|(id, f)| !state.is_allied(faction, id) && !f.at_war_with.contains(faction))
        .filter(|(_, f)| enemies.iter().all(|e| !f.allies.contains(*e)))
        // Never a pretender to our allies' lands (England for Burgundy's
        // French allies), nor a prince at war with them.
        .filter(|(id, f)| {
            me.allies
                .iter()
                .all(|a| !f.at_war_with.contains(a) && !claim_stakes(state, id, a).any())
        })
        // Lesser princes on the border of a lesser ally fighting at our
        // side: the coalition of 1337 grows as a web of neighbours and
        // in-laws around the Low Countries front (Guelders, Hainaut,
        // Brabant), not across Christendom from the Emperor's lands.
        .filter(|(id, _)| lesser(id))
        .filter(|(id, _)| {
            me.allies.iter().any(|a| {
                lesser(a)
                    && enemies.iter().any(|e| state.is_at_war(a, e))
                    && borders(state, data, id, a)
            })
        })
        .filter(|(id, _)| campaign_roll(state, id, DYNASTIC_SALT) < rules.history_permille)
        .filter_map(|(id, _)| {
            let towards_us = state.attitude(data, id, faction).0;
            let towards_enemy = enemies
                .iter()
                .map(|e| state.attitude(data, id, e).0)
                .max()?;
            Some((id, towards_us, towards_enemy))
        })
        .collect()
}

/// Alliance `faction`, at war, offers a prince who prefers it to its enemy
/// by `dynastic.margin` (G2: Edward III's in-laws of Hainaut, Brabant), in
/// about half the campaigns for each prince.
pub fn plan_dynastic_alliance(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Option<Order> {
    let rules = data.ai_alignment.as_ref()?;
    let me = state.factions.get(faction)?;
    // Off the turns of `plan_diplomacy`'s alliances ((turn + slot) % 4 == 1).
    if faction == &state.player_faction || (state.turn + slot(faction)) % 4 != 3 {
        return None;
    }
    courted_princes(state, data, rules, faction)
        .into_iter()
        .filter(|(id, towards_us, towards_enemy)| {
            // In-laws of our allies need less (Hainaut draws Brabant).
            let in_law = me
                .allies
                .iter()
                .any(|a| state.attitude(data, id, a).0 >= rules.dynastic.in_law_attitude);
            let margin = if in_law { 0 } else { rules.dynastic.margin };
            *towards_us > towards_enemy + margin && *towards_us >= rules.dynastic.min_attitude
        })
        .filter(|(id, _, _)| evaluate(state, data, faction, id, &Proposal::Alliance).accept)
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(a.0)))
        .map(|(target, _, _)| Order::ProposeAlliance {
            target: target.clone(),
        })
}

/// Money fief (G4): `faction`, at war, pensions a courted prince who does
/// not yet prefer it to its enemy by `dynastic.margin`; the gift's goodwill
/// tips the balance before the next alliance offer (Edward III's pensions
/// to Brabant and Hainaut, 1337). A pension still running is not renewed.
pub fn plan_money_fief(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Option<Order> {
    let rules = data.ai_alignment.as_ref()?;
    let me = state.factions.get(faction)?;
    // The turn before the offers of `plan_dynastic_alliance`.
    if faction == &state.player_faction || (state.turn + slot(faction)) % 4 != 2 {
        return None;
    }
    let fief = &rules.money_fief;
    if me.treasury < fief.amount * fief.treasury_multiple {
        return None;
    }
    courted_princes(state, data, rules, faction)
        .into_iter()
        .filter(|(_, towards_us, towards_enemy)| {
            *towards_us >= fief.min_attitude && *towards_us <= towards_enemy + rules.dynastic.margin
        })
        .filter(|(id, _, _)| !pensioned(state, id, faction))
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(a.0)))
        .map(|(target, _, _)| Order::SendGift {
            target: target.clone(),
            amount: fief.amount,
        })
}

/// Does `prince` still feel the goodwill of a gift from `donor`.
fn pensioned(state: &CampaignState, prince: &FactionId, donor: &FactionId) -> bool {
    state.factions.get(prince).is_some_and(|f| {
        f.modifiers.iter().any(|m| {
            &m.with == donor
                && m.expires_turn > state.turn
                && m.value > 0
                && m.reason_fr == GIFT_REASON
        })
    })
}

/// Embargoes `faction` lifts on realms now fighting its enemies (the wool
/// returns to Ghent once Flanders turns on France); the attitude case is
/// left to `plan_diplomacy`.
pub fn lift_embargoes_on_cobelligerents(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Vec<Order> {
    let Some(me) = state.factions.get(faction) else {
        return Vec::new();
    };
    me.embargoes
        .iter()
        .filter(|t| !state.is_at_war(faction, t) && state.attitude(data, faction, t).0 <= 10)
        .filter(|t| {
            state.factions.get(*t).is_some_and(|f| {
                f.at_war_with
                    .iter()
                    .any(|e| e.as_str() != "fac_rebels" && me.at_war_with.contains(e))
            })
        })
        .map(|t| Order::SetEmbargo {
            target: t.clone(),
            active: false,
        })
        .collect()
}
