# Lot DF1 — niveaux de difficulté de campagne

Branche : `feature/df1-difficulty` (worktree agent). Ne pas merger soi-même.

## État
- [x] Données `data/rules/difficulty.json` + schéma `data/schemas/difficulty_rules.schema.json` + test pytest.
- [x] `data-model` : `DifficultyRules` (défaut = miroir du fichier), chargé dans `GameData::difficulty`.
- [x] `sim-campaign::difficulty` : enum `Difficulty`, `CampaignState.difficulty` (`serde(default)`), accesseurs des leviers.
- [x] Pont : `set_difficulty`, `get_difficulty`, `get_difficulty_levels` (`campaign_sim_difficulty.rs`).
- [x] Brancher les leviers : revenus, entretien, recrutement, agitation, attitude, guerre, moral (3D + auto + prévision).
- [x] Tests Rust (`tests/df1_difficulty.rs`).
- [x] UI (à valider par le smoke) : sélecteur sur l'écran de faction, « Défi de la faction », SimFacade.pending_difficulty, sauvegardes, menu pause, smoke.
- [x] ADR 0037 (pas de codex mécaniques sur main : rien créé).

## Prochaine étape
Construire la dylib (core/build.sh), lancer le smoke Godot, vérifier cargo test complet + clippy, pytest.
