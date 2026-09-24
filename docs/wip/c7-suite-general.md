# WIP — C7 suite du général (« retinue » à la Medieval II), année de décès, mise en page de la fiche

Spéc : demande orchestrateur (lot C7 du plan `docs/design/2026-09-24-rapprochement-total-war.md`, suites de C3).
Branche : `worktree-agent-a29adc13935454195`.

## État
- [x] Données `data/retinue.json` (15 compagnons, plafond 8) + schéma `data/schemas/retinue.schema.json`.
- [x] `data-model` : `CompanionId` (`ret_`), `entities/retinue.rs`, `GameData::retinue` (optionnel), contrôle des références.
- [x] `CharacterState::{death_year, retinue}` en `#[serde(default)]`, version de sauvegarde inchangée.
- [x] Règles `sim-campaign/src/retinue.rs` : acquisition déterministe (hachage graine/tour/personnage/compagnon, sans consommer le flux aléatoire), effets via `character_effects`, prestige annuel, héritage à la mort, transfert (`Order::TransferCompanion`).
- [x] Branchements : bataille (`dynasty::on_battle_resolved`), siège et chevauchée (`siege.rs`), rançon (`chronicle::release_character`), saison en colonie amie (`turn.rs`), mort (`characters::kill`), soins (`medicine::army_wound_recovery`), solde (`economy::faction_upkeep`).
- [x] Tests Rust `core/crates/sim-campaign/tests/c7_retinue.rs` (11) + `tools/tests/test_retinue_schema.py`.
- [x] Pont `campaign_sim_retinue.rs` : `get_retinue_catalog`, `get_retinue_transfer_targets` ; `get_character` : `retinue`, `retinue_max`, `birth_year`, `death_year` ; `get_family_tree` : `death_year`. Ordre de débogage `debug_grant_companion` (smoke).
- [x] UI : `retinue_row.gd` (vignettes, infobulles, clic = confier), dates « 1310–1346 » (fiche + arbre), onglet « Suite » de l'encyclopédie (10 onglets).
- [x] Mise en page : `CharacterSheet.fit_beside` (repli en une colonne), `CourtPanel.set_max_right`, placement dans `map_ui.layout_hud` ; avertissement d'ancres corrigé.
- [ ] Smoke, capture `docs/img/c7/`.

## Prochaine étape
Smoke (22 « smoke OK »), capture `docs/img/c7/`.
