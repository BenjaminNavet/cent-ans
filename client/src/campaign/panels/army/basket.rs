//! Recruitment basket model (pure, unit-tested): port of the model half of
//! `game/scripts/map/recruit_basket.gd` (UX5-R). No rule here: ceilings (free slots, regional
//! reserve) and prices come from `game_api::recruit::RecruitView`; the treasury cap is the
//! basket's own check, as in Godot.

use std::collections::BTreeMap;

use game_api::recruit::{RecruitCard, RecruitView};

use crate::ui::kit::fr;

/// Category tabs, in display order, with their French label.
pub const CATEGORIES: [(&str, &str); 4] = [
    ("infantry", "Pied"),
    ("ranged", "Trait"),
    ("cavalry", "Cheval"),
    ("siege", "Engins"),
];

pub const SORT_LABELS: [&str; 3] = [
    "Tri : usuel",
    "Tri : prix croissant",
    "Tri : prix décroissant",
];

/// Units chosen, by unit type.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Basket {
    pub counts: BTreeMap<String, u32>,
}

fn row_of<'a>(view: &'a RecruitView, unit: &str) -> Option<&'a RecruitCard> {
    view.options.iter().find(|r| r.unit_type == unit)
}

impl Basket {
    pub fn total_count(&self) -> u32 {
        self.counts.values().sum()
    }

    pub fn total_cost(&self, view: &RecruitView) -> i64 {
        self.counts
            .iter()
            .map(|(unit, n)| i64::from(*n) * row_of(view, unit).map_or(0, |r| i64::from(r.cost)))
            .sum()
    }

    pub fn total_upkeep(&self, view: &RecruitView) -> i64 {
        self.counts
            .iter()
            .map(|(unit, n)| i64::from(*n) * row_of(view, unit).map_or(0, |r| i64::from(r.upkeep)))
            .sum()
    }

    fn room_without_treasury(&self, view: &RecruitView, card: &RecruitCard) -> i64 {
        let mut room = view.slots_free as i64 - i64::from(self.total_count());
        if card.pool_cap > 0 {
            let in_basket = i64::from(self.counts.get(&card.unit_type).copied().unwrap_or(0));
            room = room.min(i64::from(card.pool_available) - in_basket);
        }
        room
    }

    fn room_by_treasury(&self, view: &RecruitView, card: &RecruitCard) -> i64 {
        if card.cost == 0 {
            return i64::MAX;
        }
        (view.treasury - self.total_cost(view)).div_euclid(i64::from(card.cost))
    }

    /// Recruits of this type still addable: free slots, regional reserve and treasury (the whole
    /// basket must not cost more than the treasury).
    pub fn room_for(&self, view: &RecruitView, unit: &str) -> u32 {
        let Some(card) = row_of(view, unit).filter(|c| c.available) else {
            return 0;
        };
        self.room_without_treasury(view, card)
            .min(self.room_by_treasury(view, card))
            .clamp(0, i64::from(u32::MAX)) as u32
    }

    /// The treasury alone stops another recruit of this type.
    pub fn treasury_blocks(&self, view: &RecruitView, unit: &str) -> bool {
        let Some(card) = row_of(view, unit).filter(|c| c.available) else {
            return false;
        };
        self.room_without_treasury(view, card) > 0 && self.room_by_treasury(view, card) <= 0
    }

    /// Adds up to `amount`; returns how many were added.
    pub fn add(&mut self, view: &RecruitView, unit: &str, amount: u32) -> u32 {
        let added = amount.min(self.room_for(view, unit));
        if added > 0 {
            *self.counts.entry(unit.to_owned()).or_insert(0) += added;
        }
        added
    }

    pub fn remove(&mut self, unit: &str, amount: u32) -> u32 {
        let have = self.counts.get(unit).copied().unwrap_or(0);
        let removed = amount.min(have);
        if removed > 0 {
            if have == removed {
                self.counts.remove(unit);
            } else {
                self.counts.insert(unit.to_owned(), have - removed);
            }
        }
        removed
    }

    pub fn clear(&mut self) {
        self.counts.clear();
    }

    pub fn can_seal(&self, view: &RecruitView) -> bool {
        self.total_count() > 0 && self.total_cost(view) <= view.treasury
    }

    /// One entry per recruit, in the offers' order.
    pub fn order(&self, view: &RecruitView) -> Vec<String> {
        let mut out = Vec::new();
        for row in &view.options {
            for _ in 0..self.counts.get(&row.unit_type).copied().unwrap_or(0) {
                out.push(row.unit_type.clone());
            }
        }
        out
    }

    /// Keeps the basket across a refresh, bounded by the new limits.
    pub fn clamp_to(&mut self, view: &RecruitView) {
        let mut kept = BTreeMap::new();
        let mut left = view.slots_free as u32;
        for row in &view.options {
            let mut wanted = self.counts.get(&row.unit_type).copied().unwrap_or(0);
            if wanted == 0 || !row.available {
                continue;
            }
            if row.pool_cap > 0 {
                wanted = wanted.min(row.pool_available);
            }
            wanted = wanted.min(left);
            if wanted > 0 {
                kept.insert(row.unit_type.clone(), wanted);
                left -= wanted;
            }
        }
        self.counts = kept;
    }
}

/// Sort mode of the card grid.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    #[default]
    Usual,
    PriceUp,
    PriceDown,
}

impl Sort {
    pub fn next(self) -> Self {
        match self {
            Self::Usual => Self::PriceUp,
            Self::PriceUp => Self::PriceDown,
            Self::PriceDown => Self::Usual,
        }
    }

    pub fn label(self) -> &'static str {
        SORT_LABELS[self as usize]
    }
}

/// Categories present in the offers (not counting `elsewhere`).
pub fn categories_present(rows: &[RecruitCard]) -> Vec<&'static str> {
    CATEGORIES
        .iter()
        .filter(|(key, _)| {
            rows.iter()
                .any(|r| r.category == *key && r.group != "elsewhere")
        })
        .map(|(key, _)| *key)
        .collect()
}

/// Rows of the given groups, filtered by category, stably sorted.
pub fn visible_rows<'a>(
    rows: &'a [RecruitCard],
    groups: &[&str],
    category: &str,
    sort: Sort,
) -> Vec<&'a RecruitCard> {
    let mut shown: Vec<&RecruitCard> = rows
        .iter()
        .filter(|r| groups.contains(&r.group.as_str()))
        .filter(|r| category.is_empty() || r.category == category)
        .collect();
    match sort {
        Sort::Usual => {}
        Sort::PriceUp => shown.sort_by_key(|r| r.cost),
        Sort::PriceDown => shown.sort_by_key(|r| std::cmp::Reverse(r.cost)),
    }
    shown
}

/// « 120 hommes · 2 tours ».
pub fn legend_text(row: &RecruitCard) -> String {
    let turns = fr::count(i64::from(row.recruit_time_turns), "tour", None);
    if row.soldiers == 0 {
        turns
    } else {
        format!(
            "{} · {turns}",
            fr::count(i64::from(row.soldiers), "homme", None)
        )
    }
}

/// « 120 ₶ / 8 ₶ (dont import 20 ₶) ».
pub fn price_text(cost: u32, upkeep: u32, import_cost: u32) -> String {
    let mut text = format!(
        "{} / {}",
        fr::amount(i64::from(cost)),
        fr::amount(i64::from(upkeep))
    );
    if import_cost > 0 {
        text += &format!(" (dont import {})", fr::amount(i64::from(import_cost)));
    }
    text
}

/// Full wax drops (reserve available) then empty ones up to the cap.
pub fn pool_drops(available: u32, cap: u32) -> String {
    let have = available.min(cap) as usize;
    "●".repeat(have) + &"○".repeat(cap as usize - have)
}

/// Footer line of the basket (« Montre »).
pub fn footer_text(basket: &Basket, view: &RecruitView) -> String {
    if basket.total_count() == 0 {
        return format!(
            "Panier vide. Cliquer une carte : +1 ; bouton − : retirer ({}).",
            fr::count(view.slots_free as i64, "place libre", Some("places libres"))
        );
    }
    let cost = basket.total_cost(view);
    let after = view.treasury - cost;
    let n = i64::from(basket.total_count());
    format!(
        "{} {} — coût {} · solde +{} / saison · trésor après {}{}",
        fr::digits(n),
        if n > 1 { "recrues" } else { "recrue" },
        fr::amount(cost),
        fr::amount(basket.total_upkeep(view)),
        fr::amount(after),
        if after < 0 { " (insuffisant)" } else { "" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(unit: &str, cost: u32, available: bool, pool: (u32, u32)) -> RecruitCard {
        RecruitCard {
            unit_type: unit.into(),
            name: unit.into(),
            cost,
            upkeep: 5,
            available,
            reason: String::new(),
            soldiers: 100,
            recruit_time_turns: 1,
            category: "infantry".into(),
            group: if available { "ready" } else { "blocked" }.into(),
            import_cost: 0,
            pool_available: pool.0,
            pool_cap: pool.1,
            pool_seasons_to_next: -1,
            pool_label: String::new(),
            requirements: vec![],
        }
    }

    fn view(slots_free: usize, treasury: i64, options: Vec<RecruitCard>) -> RecruitView {
        RecruitView {
            settlement: "s".into(),
            settlement_name: "S".into(),
            province: "p".into(),
            province_name: "P".into(),
            own: true,
            slots: slots_free,
            slots_free,
            treasury,
            queue: vec![],
            options,
            armies: vec![],
        }
    }

    #[test]
    fn slots_pool_and_treasury_cap_the_basket() {
        let v = view(
            3,
            250,
            vec![
                card("a", 100, true, (0, 0)),
                card("b", 10, true, (1, 2)),
                card("c", 10, false, (0, 0)),
            ],
        );
        let mut basket = Basket::default();
        assert_eq!(basket.add(&v, "a", 5), 2, "treasury allows 2 of a");
        assert!(basket.treasury_blocks(&v, "a"));
        assert_eq!(basket.add(&v, "b", 5), 1, "reserve holds 1, slot left");
        assert_eq!(basket.add(&v, "b", 1), 0);
        assert_eq!(basket.add(&v, "c", 1), 0, "unavailable");
        assert_eq!(basket.total_cost(&v), 210);
        assert!(basket.can_seal(&v));
        assert_eq!(basket.order(&v), ["a", "a", "b"]);
        assert_eq!(basket.remove("a", 1), 1);
        assert_eq!(basket.total_count(), 2);
    }

    #[test]
    fn clamp_after_refresh() {
        let v = view(3, 1000, vec![card("a", 10, true, (0, 0))]);
        let mut basket = Basket::default();
        basket.add(&v, "a", 3);
        let smaller = view(1, 1000, vec![card("a", 10, true, (0, 0))]);
        basket.clamp_to(&smaller);
        assert_eq!(basket.total_count(), 1);
    }

    #[test]
    fn sorting_and_texts() {
        let rows = vec![
            card("a", 30, true, (0, 0)),
            card("b", 10, true, (0, 0)),
            card("c", 20, false, (0, 0)),
        ];
        let ready = visible_rows(&rows, &["ready", "blocked"], "", Sort::PriceUp);
        assert_eq!(
            ready
                .iter()
                .map(|r| r.unit_type.as_str())
                .collect::<Vec<_>>(),
            ["b", "c", "a"]
        );
        assert_eq!(categories_present(&rows), ["infantry"]);
        assert_eq!(pool_drops(1, 3), "●○○");
        assert_eq!(legend_text(&rows[0]), "100 hommes · 1 tour");
        assert!(price_text(120, 8, 20).contains("dont import"));
        assert_eq!(Sort::Usual.next().next().next(), Sort::Usual);
    }
}
