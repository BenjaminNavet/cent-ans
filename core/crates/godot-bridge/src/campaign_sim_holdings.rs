//! `CampaignSim` settlements quick-access API (lot HL1, touche B).
//! `docs/superpowers/specs/2026-09-27-liste-colonies-design.md` § 1.

use data_model::FactionId;
use godot::prelude::*;
use sim_campaign::holdings::{
    self, BuildOptionRow, ConstructionRow, DangerReason, ProvinceRow, SettlementRow, SiegeRow,
};

use crate::campaign_sim::{CampaignSim, Ctx};

fn danger_reason_key(reason: DangerReason) -> &'static str {
    match reason {
        DangerReason::Siege => "siege",
        DangerReason::Occupied => "occupied",
        DangerReason::RevoltCountdown => "revolt_countdown",
        DangerReason::UnrestAboveThreshold => "unrest",
    }
}

fn build_option_row_dict(option: &BuildOptionRow) -> VarDictionary {
    vdict! {
        "building" => option.building.as_str(),
        "name" => option.name.as_str(),
        "cost" => i64::from(option.cost),
        "is_upgrade" => option.is_upgrade,
    }
}

fn construction_row_dict(construction: &ConstructionRow) -> VarDictionary {
    vdict! {
        "building" => construction.building.as_str(),
        "name" => construction.name.as_str(),
        "turns_left" => i64::from(construction.turns_left),
    }
}

fn siege_row_dict(siege: &SiegeRow) -> VarDictionary {
    vdict! {
        "attacker" => siege.attacker.as_str(),
        "turns_left" => i64::from(siege.turns_left),
    }
}

fn settlement_row_dict(row: &SettlementRow) -> VarDictionary {
    let options: VarArray = row
        .options_available
        .iter()
        .map(|o| build_option_row_dict(o).to_variant())
        .collect();
    let danger_reasons: PackedStringArray = row
        .danger_reasons
        .iter()
        .map(|r| GString::from(danger_reason_key(*r)))
        .collect();
    let mut dict = vdict! {
        "id" => row.id.as_str(),
        "name" => row.name.as_str(),
        "kind" => row.kind.key(),
        "is_city" => row.is_city,
        "occupied" => row.occupied,
        "income" => row.income,
        "idle" => row.idle,
        "options_available" => &options,
        "upgrade_available" => row.upgrade_available,
        "recruit_queue_len" => row.recruit_queue_len as i64,
        "garrison_strength" => i64::from(row.garrison_strength),
        "garrison_free" => row.garrison_free,
        "endangered" => row.endangered,
        "danger_reasons" => &danger_reasons,
    };
    if let Some(construction) = &row.construction {
        dict.set("construction", &construction_row_dict(construction));
    }
    if let Some(siege) = &row.siege {
        dict.set("siege", &siege_row_dict(siege));
    }
    dict
}

fn province_row_dict(row: &ProvinceRow) -> VarDictionary {
    let settlements: VarArray = row
        .settlements
        .iter()
        .map(|s| settlement_row_dict(s).to_variant())
        .collect();
    vdict! {
        "province" => row.province.as_str(),
        "name" => row.name.as_str(),
        "income" => row.income,
        "unrest" => row.unrest,
        "revolt_seasons" => i64::from(row.revolt_seasons),
        "revolt_seasons_needed" => i64::from(row.revolt_seasons_needed),
        "revolt_threshold" => row.revolt_threshold,
        "devastation" => i64::from(row.devastation),
        "slots_busy" => row.slots_busy as i64,
        "slots_total" => row.slots_total as i64,
        "settlements" => &settlements,
    }
}

#[godot_api(secondary)]
impl CampaignSim {
    /// Settlements quick-access overview of `faction` (touche B, lot HL1):
    /// `{treasury, net_income_last_turn, settlement_income_total, count_idle,
    /// count_upgrade, count_endangered, provinces: [{province, name, income,
    /// unrest, revolt_seasons, revolt_seasons_needed, revolt_threshold,
    /// devastation, slots_busy, slots_total, settlements: [{id, name, kind,
    /// is_city, occupied, income, construction?, idle, options_available[],
    /// upgrade_available, recruit_queue_len, garrison_strength, garrison_free,
    /// siege?, endangered, danger_reasons[]}]}]}`. Empty dictionary if the
    /// faction is unknown or no campaign is loaded.
    #[func]
    fn get_holdings_overview(&self, faction: GString) -> VarDictionary {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarDictionary::new();
        };
        let Ok(faction) = FactionId::new(faction.to_string()) else {
            return VarDictionary::new();
        };
        if !state.factions.contains_key(&faction) {
            return VarDictionary::new();
        }
        let overview = holdings::holdings_overview(state, data, &faction);
        let provinces: VarArray = overview
            .provinces
            .iter()
            .map(|p| province_row_dict(p).to_variant())
            .collect();
        vdict! {
            "treasury" => overview.treasury,
            "net_income_last_turn" => overview.net_income_last_turn,
            "settlement_income_total" => overview.settlement_income_total,
            "count_idle" => overview.count_idle as i64,
            "count_upgrade" => overview.count_upgrade as i64,
            "count_endangered" => overview.count_endangered as i64,
            "provinces" => &provinces,
        }
    }
}
