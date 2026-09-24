# WIP — C7 suite du général (« retinue » à la Medieval II), année de décès, mise en page de la fiche

Spéc : demande orchestrateur (lot C7 du plan `docs/design/2026-09-24-rapprochement-total-war.md`, suites de C3).
Branche : `worktree-agent-a29adc13935454195`.

## État
- [x] Données `data/retinue.json` (15 compagnons, plafond 8) + schéma `data/schemas/retinue.schema.json`.
- [x] `data-model` : `CompanionId` (`ret_`), `entities/retinue.rs`, `GameData::retinue` (optionnel), contrôle des références.
- [x] `CharacterState::{death_year, retinue}` en `#[serde(default)]`, version de sauvegarde inchangée.
- [x] Règles `sim-campaign/src/retinue.rs` : acquisition déterministe (hachage graine/tour/personnage/compagnon, sans consommer le flux aléatoire), effets via `character_effects`, prestige annuel, héritage à la mort, transfert (`Order::TransferCompanion`).
- [x] Branchements : bataille (`dynasty::on_battle_resolved`), siège et chevauchée (`siege.rs`), rançon (`chronicle::release_character`), saison en colonie amie (`turn.rs`), mort (`characters::kill`), soins (`medicine::army_wound_recovery`), solde (`economy::faction_upkeep`).
- [ ] Tests Rust dédiés (acquisition, transmission, sérialisation ancienne sauvegarde).
- [ ] Pont godot-bridge (suite dans `get_character`, `death_year` dans fiche et arbre, catalogue).
- [ ] UI : vignettes dans la fiche, dates « 1310–1346 », encyclopédie.
- [ ] Mise en page fiche < 1500 px, avertissements d'ancres `family_tree_view.gd:418`.
- [ ] Smoke, capture `docs/img/c7/`.

## Prochaine étape
Tests Rust de `retinue.rs`, puis pont.
