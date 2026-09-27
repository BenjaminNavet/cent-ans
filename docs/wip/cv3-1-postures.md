# CV3-1 — Postures d'armée et résultats nuancés (core)

Branche : `feat/cv3-1-postures`. Spec : `docs/design/2026-09-27-campagne-vivante.md` § 1, § 3.
Orchestration : `docs/wip/cv3-campagne-vivante.md`. ADR : `docs/decisions/0094-postures-embuscade.md` (0093 pris).

## Contrat 3D (pour CV3-2) — commité dans le squelette (29297f7d)
`core/crates/sim-battle/src/setup.rs` :
- `BattleOpening { Standard (défaut), Ambush { victim: SideId } }` (serde `tag = "kind"`, snake_case :
  `{"kind":"ambush","victim":"defender"}`), `BattleSetup.opening` (serde default, omis si Standard).
- `SideSetup.forced_march: bool`, `SideSetup.entrenched: bool`, `SideSetup.start_fatigue: f64`
  (jauge 0-100 de `Unit::fatigue`, rempli depuis `postures.json` `forced_march.start_fatigue`).
- Le lot 2 lit ces champs ; il n'a pas besoin de lire `postures.json`. Aujourd'hui une embuscade
  réussie a toujours `victim: Defender` (l'embusqué attaque).

## Pont (pour CV3-4)
- `CampaignSim.get_stance_options(army_id) -> Dictionary` : clé de posture → `""` ou raison FR.
- Ordre inchangé : `submit_order({"type":"set_stance","army":…,"stance":"ambush"})` (clés
  `normal`, `raid`, `siege`, `ambush`, `forced_march`, `entrenched`) ; `stance` du dict d'armée.
- `CampaignSim.get_last_battle_outcome() -> Dictionary` (`attacker_class`, `attacker_label`,
  `defender_class`, `defender_label`, `player_class`, `player_label`, …) ; `resolve_battle` ajoute
  `outcome` au résultat. Classes : `heroic`, `decisive`, `pyrrhic`, `victory`,
  `honourable_defeat`, `disaster`, `defeat`.

## État
- [x] Squelette, règles/schémas/tests Python, `CoverMap` (data-model/src/cover.rs).
- [x] Postures : validation, effets de tour, vision, déclenchement embuscade, auto-résolution, 3D setup.
- [x] Résultats nuancés branchés dans `apply_battle_result` (XP, prestige, moral, chronique).
- [x] Pont : `get_stance_options`, `get_last_battle_outcome`.
- [x] Tests `cv3_postures.rs` (15), `cv3_outcomes.rs` (11), ancienne sauvegarde.
- [ ] Vérification finale : suite complète, pytest, smoke Godot.

## Choix
- Couvert : forêt = raster de `forest_cover.json` (`splat.png` canal B), pas `forest_kind.png`
  (part de résineux) ; marais = `wetlands.png` (max RGB) ; bocage = terrain de la province.
- Embuscade éventée : la victime attaque l'embusqué révélé (bataille normale).
- Embuscade et camp retranché interdits dans une colonie ; marche forcée et camp retranché exigent
  le mouvement plein (`army_base_grid_allowance`).
- Prestige des classes → souverain de la faction (`change_ruler_prestige`), en plus du prestige de
  victoire existant du général.

## Prochaine étape
Suite complète + smoke Godot, puis commit final « CV3-1: … ».
