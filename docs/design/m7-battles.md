# M7 — Batailles temps réel avec pause en 3D : spécification

Date : 2026-09-23. Objectif : quand une armée du joueur rencontre une armée ennemie, le joueur choisit
« Livrer bataille » (scène 3D temps réel avec pause) ou « Résolution automatique » (existant). Le résultat
revient dans la campagne. Design général : `docs/design/2026-09-23-cent-ans-design.md` § 3.4 et § 4.7.

## 1. Simulation de bataille (`core/crates/sim-battle`, pure, déterministe, sans Godot)
- Tick fixe `DT = 0.1 s` ; `BattleSim::tick(dt)` accumule et exécute autant de ticks fixes que nécessaire.
  Même setup + mêmes commandes aux mêmes ticks = même résultat (test).
- Champ de bataille 2D (x, z en mètres, ≈ 1200 × 800) avec hauteur (`height(x, z)`, collines procédurales à
  partir d'une graine et du terrain de la province : plaines, collines, forêt, marais, montagne), zones de
  forêt, de boue, un gué/rivière optionnel (si la province a une rivière). Météo tirée à l'initialisation
  selon la saison : clair, pluie (tir -40 %, arcs longs et arbalètes), brouillard (portée -30 %), neige.
- Unité (`BattleUnit`) = régiment : type (`data/unit_types`), `soldiers` vivants (depuis la force de campagne ×
  `soldiers`/100), position du centre, orientation, formation (ligne, colonne, carré/schiltron, coin pour
  cavalerie), largeur de front, état (repos, marche, charge, mêlée, tir, déroute, rallié), moral 0-100,
  fatigue 0-100, munitions, cible. Positions individuelles des soldats calculées en grille dans la formation
  avec léger bruit (pour le rendu), exposées à Godot.
- Règles : déplacement à la vitesse du type, ralenti par pente, forêt, boue, gué ; fatigue qui monte en
  marche/charge/mêlée et baisse au repos ; tir à portée (`range`), ligne de vue simplifiée, dégâts =
  `ranged` × précision (distance, météo) vs `armor`, munitions décroissantes ; mêlée au contact : dégâts
  `melee` vs `armor`, bonus de charge (cavalerie, élan), bonus de flanc (+50 %) et de dos (+100 %),
  piquiers/schiltron contre cavalerie ; pieux des archers (`stakes`) ; moral qui baisse avec les pertes, les
  flancs attaqués, la fatigue, la mort du général, les alliés en déroute proches, et monte près du
  général ; déroute sous 20, ralliement possible au-dessus de 40 loin des ennemis ; une unité qui quitte le
  champ est perdue pour la bataille (survivants rendus à la campagne).
- Général : unité portant le général de campagne, aura de moral (rayon 150 m) proportionnelle au
  commandement, bonus de charge/défense repris de `character_effects` (M4) passés au setup.
- Fin : un camp n'a plus d'unité en état de combattre, ou quitte le champ, ou 60 min simulées (défenseur
  vainqueur). Résultat : vainqueur, pertes par unité, général tué/capturé.
- IA de bataille minimale (M9 l'étoffe) : le camp IA avance en ligne, tire à portée, charge avec la
  cavalerie quand l'ennemi est à moins de 150 m, maintient les archers derrière l'infanterie. (Remplacée depuis par l'IA tactique M9, `m9-ai.md` § 2, et rééquilibrée.)
- Commandes : `move(units, point, run)`, `attack(units, target)`, `halt`, `formation(units, kind)`,
  `fire_at_will(units, bool)`, `withdraw(units)`. Validation et effets purement dans `sim-battle`.
- Tests (≥ 10) : déterminisme, tir hors portée sans effet, pluie réduit le tir, charge de flanc plus
  meurtrière, schiltron contre cavalerie, fatigue, déroute puis ralliement, fin de bataille, mort du
  général → moral, conversion des pertes vers la campagne.

## 2. Intégration campagne (`sim-campaign`)
- `CampaignState` gagne `interactive_battles: bool` (réglage joueur, défaut `true`). Quand une armée du
  joueur (attaquante ou défenseur) entre en contact pendant `resolve_movement`, et si le réglage est actif,
  la bataille est enregistrée dans `pending_battles` (`BattleRequest` existant) au lieu d'être
  auto-résolue ; l'armée s'arrête. `end_turn` retourne normalement. Les batailles d'IA contre IA restent
  auto-résolues.
- `CampaignState::battle_setup(data, index) -> BattleSetup` (unités des deux camps avec stats de
  `data/unit_types`, force, moral, expérience ; général et ses effets ; terrain de la province, saison) ;
  `resolve_pending_battle(data, index, outcome: BattleOutcome)` applique pertes, moral, vainqueur, retraite,
  XP/traits/mort du général (hooks M4 `on_battle_resolved`), événements ; `auto_resolve_pending(data,
  index)` utilise `battle_auto`. Au `end_turn` suivant, les batailles encore en attente sont
  auto-résolues d'abord. Sauvegarde : `pending_battles` sérialisées (déjà) ; `state_version` +1
  (coordonner au merge).
- Tests : bataille du joueur mise en attente, résolution externe appliquée, auto-résolution des restes.

## 3. Pont GDExtension
- `CampaignSim` : `get_pending_battles() -> [{index, attacker, defender, province, attacker_name,
  defender_name, player_side}]`, `get_battle_setup(index) -> Dictionary`, `resolve_battle(index,
  outcome_dict) -> {ok, error}`, `auto_resolve_battle(index) -> events[]`, `set_interactive_battles(bool)`.
- Nouvelle classe `BattleSim` (RefCounted) : `setup(setup_dict, seed) -> bool`, `tick(dt)`,
  `issue_command(dict) -> {ok, error}`, `get_units() -> [{id, side, type, name, soldiers, max_soldiers,
  morale, fatigue, ammo, state, formation, x, z, facing, is_general}]`, `get_soldier_transforms(side) ->
  PackedFloat32Array` (x, y, z, angle par soldat vivant, pour `MultiMeshInstance3D`),
  `get_terrain() -> {width, depth, resolution, heights: PackedFloat32Array, forests[], mud[], river?}`,
  `get_weather()`, `is_finished()`, `get_outcome() -> Dictionary` (format accepté par `resolve_battle`),
  `get_events()` (messages français : « Les chevaliers chargent », « Les archers sont à court de flèches »…).

## 4. Interface Godot (`game/scenes/battle/`)
- Dialogue en fin de tour quand `get_pending_battles` n'est pas vide : forces en présence, terrain, météo
  prévue, boutons « Livrer bataille » / « Résolution automatique ».
- Scène `battle.tscn` : terrain maillé depuis `get_terrain` (couleurs herbe/forêt/boue/eau, arbres low-poly
  instanciés), soldats en `MultiMeshInstance3D` par camp et par catégorie (mesh low-poly simple : fantassin,
  cavalier, archer ; couleur de faction), bannières d'unité (couleur + icône de catégorie) au-dessus de
  chaque régiment, caméra RTS (WASD, molette, rotation Q/E, bords d'écran).
- Contrôles : clic gauche sélection (rectangle, Maj pour ajouter), clic droit déplacer / attaquer (sur une
  unité ennemie), glisser-droit pour orienter la ligne, touches formation (F), tir à volonté (G), halte (H),
  espace = pause, 1/2/3 = vitesse ×1/×2/×4. Ordres possibles en pause.
- HUD parchemin : cartes d'unité en bas (effectif, moral, fatigue, munitions), barre de rapport de forces,
  journal de bataille, horloge, météo. Écran de fin : vainqueur, pertes, bouton « Retour à la campagne »
  qui appelle `resolve_battle` puis revient sur la carte.
- Smoke test : setup d'une bataille réelle (France vs Angleterre), 2 000 ticks headless via `BattleSim`,
  fin atteinte, `resolve_battle` accepté ; chargement de `battle.tscn` et 60 frames sans erreur.
- Captures : `docs/img/godot-battle.png`, `docs/img/godot-battle-dialog.png`.

## 5. Critères de fin
Tests Rust verts, smoke vert, captures relues, 60 fps visés pour 2 × 20 unités de 120 soldats sur Apple
Silicon (mesurer et consigner dans `docs/godot-map.md`), docs à jour (`status`, `roadmap`, `data-model`).

## F1 (v2) — armées alliées
Les armées de la province de la rencontre, de la même faction ou alliées et en guerre contre l'adversaire,
rejoignent leur camp : auto-résolution (régiments concaténés, chacun avec les techs de sa faction, un seul
général commandant = meilleur commandement, ravitaillement moyen pondéré) et `battle_setup` (camp portant
l'id et la faction de l'armée de tête). Les pertes (auto-résolues ou issues de la bataille 3D, même ordre
de régiments) sont réparties sur chaque armée, seul le commandant peut être capturé et tous les vaincus
retraitent. La bataille est différée si le joueur est dans l'une des coalitions. Les assauts de siège
restent à deux.
