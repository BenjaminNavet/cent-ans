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
- [x] Pont GDExtension (`get_terrain().decor`, `get_camps()`).
- [x] Rendu Godot `battle_decor.gd` (MultiMesh, LOD), vergers (`_plant_orchards`), parcelles
  peintes (`decor_fields` lu par `battle_ground`/`battle_grass`), fossé, feux
  (`battle_camp_fire.gdshader`), camp pillé, pieux (`battle_volleys._stake_mesh`) ;
  `--no-ep6-decor` (A/B), `--province=<id>` (paysage d'une province, captures).
- [x] ADR 0061 (`docs/decisions/0061-villages-et-decor-du-champ-de-bataille.md`).
- [x] Import Godot, premier lancement sans erreur de script ; retouches kit (roue ajourée du
  moulin, chevaux nus) commitées.
- [x] Grille des emprises du décor (`sim/obstacles.rs::DecorGrid`, cellules de 64 m) ; l'IA prend
  les zones du décor comme couverts (`ai.rs`, test `a_manor_before_the_line_is_a_defensive_cover`).
- [x] Fusion de main (EP8, d0be0f2b) ; feux des camps → `BattleScene.add_smoke_source` (au plus
  `camp_fires_per_side` par camp, pas sous la pluie/neige), `staging.auto_campfires = false` dès
  que le décor a des camps ; camp pillé → sources « column » d'EP8. Fumée vérifiée en capture.
- [x] Banc EP1 A/B (ci-dessous), captures `docs/img/ep6/` (`vue_hameaux_moulin`, `vue_hameau_rue_camp`,
  `camp_feux_fumee`, `melee_hameau`, `guyenne_moulin_vignes`). `--decor-plan=<fichier>` (Godot) passe
  un plan JSON à la simulation (essais EP7, captures).
- [ ] Vérifications finales (smoke Godot, pytest, cargo workspace) après la dernière fusion.

## Mesures (banc `tools/bench_ep1.sh … --units=63 --bench-at=90`, 15 068 soldats, palier epic)
Mac M4 Pro, machine partagée (charge 5-15) ; A = `--no-ep6-decor` (rendu du décor coupé, règles
du cœur identiques), B = décor rendu. Guyenne (vignoble, pire cas : 67 bâtiments, 3 255 segments de
vigne, 42 tentes, 57 chevaux).

| Config | i/s moy. | p95 ms | primitives | appels |
|---|---|---|---|---|
| B décor, EP8 actif | 29,9 · 29,8 | 37,5 · 37,9 | 3,61 M | 1 408 |
| A sans décor, EP8 actif | 30,1 · 30,1 | 37,5 · 37,6 | 3,50 M | 1 321 |
| B décor, `--no-ep8` | 26,7 | 45,5 | 3,64 M | 1 403 |
| A sans décor, `--no-ep8` | 27,5 | 42,7 | 3,53 M | 1 317 |
| B décor, Picardie (openfield) | 30,4 | 36,1 | 2,58 M | 1 312 |

Lecture : le décor coûte ~+3 % de primitives, ~+87 appels, et 0 à 3 % d'i/s (dans le bruit).
Toutes les mesures plafonnent vers 30 i/s ce soir, décor ou non (EP1 mesurait ~57 le matin, écran à
60 Hz) : écart d'environnement (charge, affichage), pas du lot ; la cible de 40 i/s n'a pas pu être
revérifiée dans ces conditions.

## API de pose explicite (pour EP7)

Deux voies, au choix, toutes deux dans le cœur (`core/crates/sim-battle/src/decor.rs`) :

1. **Plan JSON** (recommandé pour les cartes historiques) : `BattleSetup::decor_plan:
   Option<DecorPlan>`, appliqué par `BattleSim::new_scaled` après le décor procédural. Schéma
   `data/schemas/battle_decor_plan.schema.json`, exemple commenté
   `data/battle_maps/decor_plan_example.json`. `{"description", "clear": bool, "items": [...]}` ;
   `clear: true` retire d'abord le décor procédural (les camps procéduraux restent sauf si le plan
   pose les siens). Éléments (`type`) :
   - `building` {kind: cottage|timbered|barn|church|stone|windmill|watermill|manor, x, z, yaw, length?, width?}
   - `windmill` {x, z, yaw?, mound?} — moulin sur pivot, butte levée dans la grille des hauteurs
   - `watermill` {x, z, yaw?} — roue vers `(-sin yaw, cos yaw)`
   - `church` {x, z, yaw?} — église + cimetière clos (zone `church`)
   - `manor` {x, z, yaw?, moat?} — maison forte, cour, grange, puits, fossé en eau
   - `hamlet` {layout: street|green|farmstead, x, z, yaw?, houses? (0 = usuel), seed?} — disposé
     autour des routes et de l'eau ; `street` suit la route la plus proche de (x, z)
   - `plot` {kind: orchard|vineyard|ploughland|meadow|hamlet|church|manor|farmstead|camp, x, z, length, width, yaw?, state?
     (ploughed|sown|crop|stubble)} — les prés reçoivent les meules de la saison
   - `prop` {kind: haystack|cart|tent|pavilion|wagon|campfire|horse_line|graves|well|woodpile, x, z, yaw?}
   - `camp` {side: attacker|defender, x, z, yaw? (front vers l'ennemi), seed?} — remplace le camp du camp
   - `hedge` {a: [x, z], b: [x, z]}
2. **Appels Rust** sur `Battlefield` : `clear_decor()`, `place_building(kind, x, z, yaw, length,
   width) -> usize`, `place_windmill(x, z, yaw, mound)`, `place_watermill(x, z, yaw)`,
   `place_church(x, z, yaw)`, `place_manor(x, z, yaw, moat)`, `place_hamlet(layout, x, z, yaw,
   houses, seed) -> bool`, `place_plot(kind, x, z, length, width, yaw, state)`, `place_prop(kind,
   x, z, yaw)`, `place_camp(side, x, z, yaw, seed) -> bool`, `apply_decor_plan(&plan)`.
   Pose à la main = sans contrôle (sauf `place_hamlet`/`place_camp`, qui évitent routes et eau).
   Après modification du champ d'une simulation déjà créée, passer par `BattleSim::field_mut()`
   (remet à zéro la grille des emprises).
- Requêtes d'effets : `decor_area_at`, `decor_effect_at`, `decor_speed_factor`, `decor_cover`,
  `decor_defense`, `decor_breaks_charge`, `decor_footprints_near`. Rendu : aucun travail côté
  Godot, `get_terrain().decor` porte tout (le rendu relit le décor tel quel).
- Coordonnées en mètres dans le repère du champ (`FieldSize`, 0..width × 0..depth ; attaquant côté
  z petit). `yaw` en radians : longueur le long de `(cos, sin)`, façade vers `(-sin, cos)`.

## Prochaine étape
Vérifications finales (cargo fmt/clippy/test workspace, build.sh, pytest, ruff, import, smoke Godot)
puis rapport.
