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

## Validation (2026-09-25, reprise après interruption de l'agent)
- cargo fmt / clippy -D warnings / cargo test : verts. pytest tools : 430 passés.
- Smoke Godot : toutes les vérifications DF1 passent. Échecs hors lot, présents aussi sur main :
  « music playlist too short » (avant 580fc208), puis à 580fc208 plantage « Message queue out of
  memory » (Control::_update_minimum_size) dans `_run_campaign_loop`, reproduit sur main seul.

## Prochaine étape
Lot fusionné dans main. Suites possibles : équilibrage des valeurs après parties de test ;
codex « Difficulté » quand le codex des mécaniques sera stable.
