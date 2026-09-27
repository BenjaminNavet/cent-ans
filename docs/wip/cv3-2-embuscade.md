# CV3-2 — Ouverture d'embuscade, marche forcée, camp retranché (bataille 3D)

Branche : `feat/cv3-2-ambush-battle` (base `main` 9bdd69f9, contient CV3-1 5096da0b).
Spec : `docs/design/2026-09-27-campagne-vivante.md` § 1.2-1.3 ; ADR 0094 ; contrat CV3-1 dans
`core/crates/sim-battle/src/setup.rs` (inchangé).

## Fait
- Règles `data/rules/battle_opening.json` + schéma `battle_opening_rules.schema.json` + test
  Python `tools/tests/test_battle_opening_schema.py` ; `sim_battle::OpeningRules` (`src/opening.rs`).
- `ObstacleKind::Palisade` (site.rs) : ralentit (facteurs en données), couvre des traits, brise les
  charges ; diviseur de pertes en mêlée pour le défenseur derrière (`palisade_defense`).
- `src/sim/opening.rs` : `apply_opening` (après `deploy`) — fatigue de départ, colonne de la victime
  le long de la plus longue route (sinon axe long), ordre avant-garde → bataille → arrière-garde,
  zones de flanc de l'embusqué (score forêt/haies, 1 ou 2 flancs), camp retranché (pieux plantés,
  palissade) ; `can_deploy`, `deployment_zones`, `ambush_layout`, `palisades` ; journal
  (`log_opening`).
- `deployment.rs` : `begin_deployment` refuse si le camp du joueur ne peut se déployer, pas
  d'`ai_deploy` pour colonne / marche forcée / embusqué ; `deployable` teste toutes les zones.

## Reste
- Tests `sim-battle/tests/cv3_ambush.rs`.
- Pont : `get_deployment_zone` vide si pas de déploiement, `get_deployment_zones`, `get_opening`.
- Godot : dialogue (« Embuscade ! », marche forcée, camp retranché), badge HUD, zones multiples,
  palissade dessinée (battle_village.gd), minicarte ; étape smoke.
- Vérification finale (fmt/clippy/test, pytest, build.sh, import, smoke).
