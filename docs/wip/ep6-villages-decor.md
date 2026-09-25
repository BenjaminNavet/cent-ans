# Lot EP6 — Villages et décor du champ de bataille (chantier « batailles épiques »)

Branche `ep6-villages-decor` (worktree `.claude/worktrees/agent-afdabf24728dfdac8`). Suivi du
chantier : `docs/wip/epic.md`. Dépend d'EP1 (taille du champ) et d'EP3 (eau, ponts, routes).

## Objet
Le champ de bataille doit ressembler à une campagne du XIVe siècle : hameaux en rue ou groupés
autour de l'église, fermes isolées, moulin à vent sur butte, moulin à eau, église et cimetière clos,
manoir à fossé, vignes, vergers, labours, prés et meules, charrettes, haies bocagères ; camp et
convoi de bagages derrière chaque armée ; pieux des archers.

## Conception
- Cœur : `core/crates/sim-battle/src/decor.rs` (règles `DecorRules`, types, génération
  procédurale `Battlefield::lay_decor`, pose à la main pour EP7 `place_*` et `DecorPlan`, requêtes
  d'effets) ; `sim/camp.rs` (pillage du camp) ; données `data/rules/battle_decor.json` + schéma
  `battle_decor_rules.schema.json` ; plan de décor `battle_decor_plan.schema.json`.
- Composition : la province choisit un paysage (`profiles` : vignoble, bocage, openfield du Nord,
  Angleterre, Midi…), sinon le terrain ; la saison règle meules, charrettes, état des labours,
  vignes feuillues, vergers en fleurs.
- Règles : zones (hameau, cimetière, manoir, ferme, verger, vigne, labours, pré, camp) → couvert,
  vitesse (labours boueux sous la pluie), défense en mêlée, charges brisées ; bâtiments et mobilier
  solides pour les figurines (comme BR3).
- Camp : un ennemi en état de combattre dans un camp sans garde le pille en `loot_seconds` ;
  alarme puis perte de moral de toute l'armée ; `SideResult::baggage_lost`.
- Kit Blender : nouvelles recettes (moulin à eau, tentes, pavillons, meules, chariots, feux, mur de
  cimetière, porche, tombes, rangs de vigne, chevaux au piquet) — sous-agent.

## État
- [x] Squelette : données + schéma + test pytest, `decor.rs` (types, API), `sim/camp.rs`,
  `tests/ep6_decor.rs` (tests désactivés), champ `Battlefield::decor`, `BattleSetup::decor_plan`,
  `SideResult::baggage_lost`, `HouseKind::{Stone, Windmill, Watermill, Manor}`.
- [x] Génération procédurale (`decor_gen.rs` : hameaux en rue / groupés, fermes, moulins,
  église, manoir, parcelles, meules, charrettes, haies, camps et convoi) + tests.
- [x] Règles : effets des zones (vitesse, couvert, défense, charges), figurines hors des
  bâtiments, pillage du camp + tests (`tests/ep6_decor.rs`, 8 tests verts).
- [x] Pose à la main (EP7) + `DecorPlan` + exemple `data/battle_maps/decor_plan_example.json`.
- [x] Kit Blender (sous-agent) : 27 modèles (watermill, tent, pavilion, haystack, wagon,
  campfire, wall_run, lychgate, graves, vine_row, horse) exportés ; aperçus `docs/img/ep6/kit_*`.
- [ ] Pont GDExtension (`get_terrain().decor`, `get_camps()`).
- [ ] Import Godot des nouveaux modèles.
- [ ] Rendu Godot `battle_decor.gd` (MultiMesh, LOD), vergers, labours, fossé, feux, pieux.
- [ ] Banc EP1 avant/après, captures `docs/img/ep6/`, fusion de main, vérifications finales.

## Prochaine étape
Suite complète `cargo test` (régressions B6/EP3 ?), puis pont GDExtension et rendu Godot.
