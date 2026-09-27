# CV3-2 — Ouverture d'embuscade, marche forcée, camp retranché (bataille 3D)

Branche : `feat/cv3-2-ambush-battle` (base `main` 9bdd69f9, contient CV3-1 5096da0b).
Spec : `docs/design/2026-09-27-campagne-vivante.md` § 1.2-1.3 ; ADR 0094 (§ « Suite CV3-2 ») ;
contrat CV3-1 de `core/crates/sim-battle/src/setup.rs` inchangé ; format de rejeu inchangé.

## État : terminé
- Règles `data/rules/battle_opening.json` + schéma `battle_opening_rules.schema.json` + test
  Python ; `sim_battle::OpeningRules` et `opening::column_path` (`src/opening.rs`).
- `ObstacleKind::Palisade` (site.rs) : ralentit (facteurs en données), couvre des traits, brise les
  charges ; diviseur de pertes en mêlée du défenseur juste derrière (`palisade_defense`).
- `src/sim/opening.rs` : `apply_opening` (après `deploy`) — fatigue de départ, colonne de la victime
  (plus longue route, sinon axe long ; avant-garde → bataille → arrière-garde), zones de flanc de
  l'embusqué (score forêt/haies, 1 ou 2 flancs), camp retranché (pieux, palissade) ; `can_deploy`,
  `deployment_zones`, `ambush_layout`, `palisades` ; journal (`log_opening`).
- `deployment.rs` : `begin_deployment` → `false` si le camp du joueur ne peut se déployer ; pas
  d'`ai_deploy` pour colonne / marche forcée / embusqué ; `deployable` teste toutes les zones ; IA
  d'un camp retranché sans recul vers les hauteurs.
- Pont : `get_deployment_zone` vide sans déploiement, `get_deployment_zones`, `get_opening`.
- Godot : titre/sous-titre « Embuscade ! » et mentions des postures (pre_battle_dialog.gd), badge
  HUD (battle_hud.gd `set_opening`), zones multiples (deployment_controller.gd), palissade dessinée
  (battle_village.gd) et sur la minicarte ; étape smoke `_check_ambush_opening_cv3`.
- Tests : `sim-battle/tests/cv3_ambush.rs` (11) ; cargo fmt/clippy/test, pytest, build, smoke OK.

## Limites / suites possibles
- Zones de flanc = rectangles alignés sur les axes de la carte (pas orientés le long d'une route en
  biais) ; le couvert est échantillonné (forêts, haies/clôtures), pas les lisières du décor EP6.
- La palissade suit l'axe x (lignes de bataille standard) ; un camp retranché du joueur peut se
  redéployer loin de sa palissade pendant F5c.
- Pas de capture visuelle faite (rendu de la palissade et du badge à contrôler par la session
  principale).
