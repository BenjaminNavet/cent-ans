# FK1 — cœur Rust de la carte vivante

Branche `feat/fk1-core`. Spec `docs/design/2026-09-29-carte-vivante-folk.md` §§ 2.1, 6, 7 ;
ADR 0122 ; note chantier `docs/wip/fk.md` (non modifiée ici pour éviter les conflits de vague).

## État
- [x] `data-model` : `Event.map_scene: Option<SceneKind>`, `Event.presentation: Option<EventPresentation>`,
  `Event::presentation()` (défaut `map` pour `random` + portée province, `dialog` sinon).
  `SceneKind` vit dans `data-model` (réexporté par `sim_campaign::map_scenes`).
- [x] `data/rules/map_scenes.json` + `MapSceneRules` (`GameData.map_scene_rules`, défaut = fichier),
  schéma `map_scenes_rules.schema.json`, enum `map_scene` / `presentation` dans `event.schema.json`,
  pytest `tools/tests/test_map_scenes_schema.py`.
- [x] 23 événements tagués `map_scene`.
- [x] Chronique : `ChronicleState.recent_scenes` (`#[serde(default)]`), rempli au déclenchement
  (province, sinon capitale du décideur) et par la vague de peste noire ; purgé à la fin de durée.
- [x] Expiration : `ai_affordable_choice_among` (options offertes), ADR 0122 ; test M10 renommé ;
  infobulle et codex mis à jour.
- [x] `map_scenes()` : révolte, dévastation, famine (hiver + dévastation > 70), maladie (santé < 30),
  siège, chantier, recrutement, événements récents.
- [x] Pont : `get_map_scenes()`, `get_map_scene_rules()`, `presentation` dans `get_pending_decisions()`.
- [x] Tests `fk_map_scenes.rs` (5 verts, taux d'incidents #[ignore] jusqu'à FK5).

## Prochaine étape
Relecture, fusion dans `integration/fk`.
