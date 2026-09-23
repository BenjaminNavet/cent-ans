# M3 — sim-campaign cities & economy: état d'avancement

## Fait
- `data/factions/fac_rebels.json` : faction virtuelle non jouable (cible des révoltes).
- `state.rs` : STATE_VERSION=2, `ProvinceState.buildings/construction/revolt_seasons`,
  `FactionState.tax_rate/goods/army_upkeep_last_turn/building_upkeep_last_turn/projected_income`,
  `FactionSummary` + 4 champs, struct `Construction`.
- `buildings.rs` : `EffectTotals`/`EffectValue`, `effects_of`, `capacity`, `province_building_upkeep`,
  `buildable`, `BuildOption`, `province_city`/`ProvinceCity`, `resolve_construction` (phase de tour),
  `goods_map`, `fortification_level` (remplace la constante de siège).
- `population.rs` : `resolve_population` (croissance/santé/richesse/satisfaction/mécontentement),
  révolte (2 saisons > 75 pondéré → événement + garnison -25% ; > 90 → contrôleur `fac_rebels`),
  peste (santé moyenne < 30), famine hivernale (dévastation > 70).
- `economy.rs` : `TaxRate` (Low/Normal/High), `FactionEconomy`, `province_income_effective`,
  `faction_income_effective`, `faction_economy`, `resolve_goods`, upkeep de bâtiments dans
  `resolve_economy` (nouveaux champs cache sur `FactionState`).
- `orders.rs` : `Order::Build/CancelBuild/SetTaxRate` + erreurs associées.
- `siege.rs` : durée de siège via `fortification_level` (base + effets bâtiments).
- `turn.rs` : nouvelles phases (constructions, biens, population) dans `end_turn`.
- `save.rs` : message de version en français.
- Workspace complet compile (`cargo build --workspace`), y compris `godot-bridge`.
- Tests existants mis à jour (nombre de factions, siège, save/load v2, revenu effectif).

## Prochaine étape
- Écrire les ≥12 nouveaux tests (croissance, construction, taxes, révolte, biens, peste,
  save/load v1 refusé, déterminisme 20 tours, 40 tours sans panique).
- `cargo fmt` + `cargo clippy --workspace --all-targets -- -D warnings`.
- Mesurer et noter le revenu France (Normal) dans le rapport final.
- Supprimer ce fichier à la fin.
