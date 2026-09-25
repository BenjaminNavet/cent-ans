# Lot DF1 — niveaux de difficulté de campagne

Branche : `feature/df1-difficulty` (worktree agent). Ne pas merger soi-même.

## État
- [x] Données `data/rules/difficulty.json` + schéma `data/schemas/difficulty_rules.schema.json` + test pytest.
- [x] `data-model` : `DifficultyRules` (défaut = miroir du fichier), chargé dans `GameData::difficulty`.
- [x] `sim-campaign::difficulty` : enum `Difficulty`, `CampaignState.difficulty` (`serde(default)`), accesseurs des leviers.
- [x] Pont : `set_difficulty`, `get_difficulty`, `get_difficulty_levels` (`campaign_sim_difficulty.rs`).
- [x] Brancher les leviers : revenus, entretien, recrutement, agitation, attitude, guerre, moral (3D + auto + prévision).
- [x] Tests Rust (`tests/df1_difficulty.rs`).
- [ ] UI : sélecteur sur l'écran de faction, « Défi de la faction », SimFacade.pending_difficulty, sauvegardes, menu pause, smoke.
- [ ] ADR 0037, codex mécaniques si présent.

## Prochaine étape
UI Godot (faction_select.gd, sim_facade.gd, mock, save_slots.gd, pause), smoke, puis ADR 0037. Vérifier cargo test complet + clippy.
