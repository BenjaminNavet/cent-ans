//! Characters: governors, generals, skill points and marriages.

use std::collections::BTreeSet;

use data_model::{CharacterId, FactionId, ProvinceId, SkillBranch, SkillId};
use sim_campaign::{Order, Season};

use super::Context;

/// The faction's characters for the turn: generals, governors, skills and
/// (once a year) a marriage.
pub(super) fn plan_characters(ctx: &Context, orders: &mut Vec<Order>) {
    let state = ctx.state;
    // Characters already commanding an army or governing a province.
    let mut busy: BTreeSet<CharacterId> = state
        .characters
        .iter()
        .filter(|(_, c)| c.army.is_some() || c.governor_of.is_some())
        .map(|(id, _)| id.clone())
        .collect();
    assign_generals(ctx, &mut busy, orders);
    assign_governors(ctx, &busy, orders);
    learn_skills(ctx, orders);
    // Marriages: once a year.
    if state.season == Season::Spring {
        arrange_marriage(ctx, orders);
    }
}

/// A living, free adult of the faction.
fn available(ctx: &Context, id: &CharacterId) -> bool {
    let year = ctx.state.year();
    ctx.state
        .characters
        .get(id)
        .is_some_and(|c| c.alive && !c.captive && &c.faction == ctx.faction && c.is_major(year))
}

/// Generals: the best available commander standing with each leaderless army.
fn assign_generals(ctx: &Context, busy: &mut BTreeSet<CharacterId>, orders: &mut Vec<Order>) {
    let state = ctx.state;
    for (army_id, army) in state
        .armies
        .iter()
        .filter(|(_, a)| &a.faction == ctx.faction)
    {
        if army.general.is_some() {
            continue;
        }
        let army_province = state.army_province(ctx.data, army);
        let best = state
            .characters
            .iter()
            .filter(|(id, c)| {
                available(ctx, id)
                    && !busy.contains(*id)
                    && c.location.is_some()
                    && c.location == army_province
            })
            .max_by_key(|(id, c)| (c.skills.command, std::cmp::Reverse((*id).clone())))
            .map(|(id, _)| id.clone());
        if let Some(general) = best {
            busy.insert(general.clone());
            orders.push(Order::AssignGeneral {
                army: army_id.clone(),
                character: general,
            });
        }
    }
}

/// Governors: best administrators to the richest ungoverned provinces.
fn assign_governors(ctx: &Context, busy: &BTreeSet<CharacterId>, orders: &mut Vec<Order>) {
    let state = ctx.state;
    let ruler = state.factions[ctx.faction].ruler.as_ref();
    let mut provinces: Vec<(ProvinceId, i64)> = state
        .provinces
        .keys()
        .filter(|id| ctx.owns(id) && state.province_governor(id).is_none())
        .map(|id| (id.clone(), ctx.province_income(id) as i64))
        .collect();
    provinces.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut candidates: Vec<(CharacterId, u8)> = state
        .characters
        .iter()
        .filter(|(id, _)| available(ctx, id) && !busy.contains(*id) && Some(*id) != ruler)
        .map(|(id, c)| (id.clone(), c.skills.governance))
        .collect();
    candidates.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    for ((province, _), (character, _)) in provinces.into_iter().zip(candidates).take(3) {
        orders.push(Order::AssignGovernor {
            province,
            character,
        });
    }
}

/// Skill points: the branch of the character's job first, else the cheapest
/// tier on offer.
fn learn_skills(ctx: &Context, orders: &mut Vec<Order>) {
    let (state, data) = (ctx.state, ctx.data);
    for (id, c) in state
        .characters
        .iter()
        .filter(|(_, c)| c.alive && &c.faction == ctx.faction && c.skill_points > 0)
    {
        let branch = if c.army.is_some() {
            SkillBranch::Command
        } else if c.governor_of.is_some() {
            SkillBranch::Governance
        } else {
            SkillBranch::Court
        };
        let learnable: Vec<SkillId> = state
            .learnable_skills(data, id)
            .into_iter()
            .filter(|s| data.skills.get(s).is_some_and(|d| d.cost <= c.skill_points))
            .collect();
        let pick = learnable
            .iter()
            .filter(|s| data.skills[*s].branch == branch)
            .min_by_key(|s| (data.skills[*s].tier, (*s).clone()))
            .or_else(|| {
                learnable
                    .iter()
                    .min_by_key(|s| (data.skills[*s].tier, (*s).clone()))
            });
        if let Some(skill) = pick {
            orders.push(Order::LearnSkill {
                character: id.clone(),
                skill: skill.clone(),
            });
        }
    }
}

/// One unmarried adult of the ruling house marries.
fn arrange_marriage(ctx: &Context, orders: &mut Vec<Order>) {
    let state = ctx.state;
    let year = state.year();
    let me = &state.factions[ctx.faction];
    let ruler = me.ruler.as_ref();
    let Some(house) = ruler
        .and_then(|r| state.characters.get(r))
        .map(|r| r.house.clone())
    else {
        return;
    };
    // F4: the ruler and the heir first (a dynasty needs sons), then the
    // eldest of the house.
    let heir = me.heir.as_ref();
    let single = state
        .characters
        .iter()
        .filter(|(id, c)| {
            available(ctx, id)
                && c.spouse.is_none()
                && c.house == house
                && c.age(year) >= 16
                && c.age(year) <= 45
        })
        .min_by_key(|(id, c)| {
            let rank = if Some(*id) == ruler {
                0
            } else if Some(*id) == heir {
                1
            } else {
                2
            };
            (rank, c.birth_year, (*id).clone())
        })
        .map(|(id, _)| id.clone());
    let Some(single) = single else {
        return;
    };
    match marriage_partner(ctx, &single) {
        Some((spouse, spouse_faction)) if &spouse_faction == ctx.faction => {
            orders.push(Order::ProposeMarriage {
                character: single,
                spouse,
            });
        }
        Some((spouse, spouse_faction)) => orders.push(Order::ProposeFactionMarriage {
            target: spouse_faction,
            character: single,
            spouse,
        }),
        None => {}
    }
}

/// Best spouse for `single` (F4): a partner of child-bearing age close in
/// years, preferably from a ruling house of an ally or of a friendly realm
/// (diplomatic marriage), who would accept; the player receives an offer.
fn marriage_partner(ctx: &Context, single: &CharacterId) -> Option<(CharacterId, FactionId)> {
    let state = ctx.state;
    let data = ctx.data;
    let year = state.year();
    let me = state.characters.get(single)?;
    let ruling = |c: &sim_campaign::CharacterState| {
        state
            .factions
            .get(&c.faction)
            .and_then(|f| f.ruler.as_ref())
            .and_then(|r| state.characters.get(r))
            .is_some_and(|r| r.house == c.house)
    };
    state
        .marriage_candidates(data, single)
        .into_iter()
        .filter_map(|id| state.characters.get(&id).map(|c| (id, c)))
        .filter(|(_, c)| !c.captive && !c.faction.is_rebels())
        .filter(|(_, c)| (c.age(year) - me.age(year)).abs() <= 15)
        .filter(|(_, c)| {
            let wife = if c.sex == data_model::Sex::Female {
                c
            } else {
                me
            };
            wife.age(year) <= 35
        })
        .filter_map(|(id, c)| {
            let own = &c.faction == ctx.faction;
            let attitude = if own {
                0
            } else {
                state.attitude(data, ctx.faction, &c.faction).0
            };
            if !own && (attitude < 0 || state.is_at_war(ctx.faction, &c.faction)) {
                return None;
            }
            let mut value = c.prestige + attitude;
            if !own && ruling(c) {
                value += 40;
            }
            if state.is_allied(ctx.faction, &c.faction) && !own {
                value += 20;
            }
            Some((value, id, c.faction.clone()))
        })
        .filter(|(_, id, faction)| {
            faction == ctx.faction
                || faction == &state.player_faction
                || sim_campaign::negotiation::evaluate_treaty(
                    state,
                    data,
                    ctx.faction,
                    faction,
                    &[sim_campaign::negotiation::Article::Marriage {
                        character: single.clone(),
                        spouse: id.clone(),
                    }],
                )
                .accept
        })
        .max_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)))
        .map(|(_, id, faction)| (id, faction))
}
