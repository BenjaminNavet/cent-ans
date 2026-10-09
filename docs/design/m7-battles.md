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
  Lot B5 (`sim-battle/src/site.rs`) : le site de campagne complète le champ, tiré d'un flux dérivé de la
  graine (les tirages d'avant B5 sont inchangés) — sol de la saison (sec, détrempé : boue en plus, enneigé :
  marche ×0,9 sans chute de neige), côte sur un flanc si la province est côtière (plage de sable ×0,85, mer
  hors du champ), mares du marais (eau peu profonde) et fossés de drainage, haies du bocage, village ou ferme
  (`BattleSetup.village` : forcé, interdit ou tiré selon le terrain ; jamais en siège) avec courtils entourés
  de haies et de clôtures. Haie : couvert contre les traits (×0,6 derrière une haie que le tir traverse),
  charge de cavalerie brisée (pas d'impact) ; fossé : charge brisée ; haie, clôture, fossé ralentissent le
  franchissement (surtout à cheval) ; village : couvert ×0,6, ruelles lentes, charge brisée.
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
  `get_terrain() -> {width, depth, resolution, heights: PackedFloat32Array, forests[], mud[], river?}`
  (B5 : plus `terrain, season, ground, ground_label, woodland, pools[], obstacles[], coast?, village?`),
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

## 6. Ordres du chef (F10b)
Cri de guerre, pas de quartier, pied à terre, pavois, ralliement : `Command::LeaderOrder`, catalogue
`data/battle_orders/`, barre d'ordres en bataille. Spécification : `docs/design/battle-orders.md`.

## F5b — HUD de bataille (audit UI § 3.2)
- Cartes d'unités compactes (`game/scripts/battle/unit_card.gd`, 71 px au lieu de 142) : icône de classe
  (`IconLibrary`), effectif, barres fines moral / fatigue / munitions, état ; formation et chiffres détaillés
  dans l'infobulle. Le nom tient sur deux lignes, coupé seulement entre deux mots (police réduite jusqu'à
  8 px, puis « … » après le dernier mot entier).
- Cartes rangées par « bataille » (`battle_groups.gd`) : avant-garde (cavalerie), bataille (infanterie et
  tireurs), arrière-garde (engins de siège, et unités `reserve` si la simulation l'expose un jour).
- Raccourcis : **Ctrl+1..9** (ou Cmd+1..9) enregistre la sélection en groupe, **1..9** la rappelle (deux
  appuis rapprochés : caméra centrée sur le groupe) ; touches physiques, donc AZERTY compris. Les vitesses
  quittent donc 1/2/3 : boutons-icônes en bas à droite (pause, ×1, ×2, ×4 ; bouton actif cerclé d'or),
  **Espace** pause, **+ / −** vitesse. **F1** : aide de bataille (plus de ligne d'aide permanente).
- Minicarte cliquable (`battle_minimap.gd`) : relief, bois, boues, rivière, régiments en points aux
  couleurs des camps, cadre de la caméra ; clic ou glisser = caméra ; retournée pour que le camp du joueur
  soit en bas. Capture : `docs/img/godot-battle-f5.png`.

## F5 — finition de la simulation (F5a, Rust)
- **Collisions amies** (`sim/separation.rs`) : deux régiments du même camp à l'arrêt dont les
  rectangles se chevauchent s'écartent (jeu de 1 m, `FRIEND_GAP`), 3 m/s au plus, déterministe. Un
  régiment qui marche, charge ou poursuit une cible traverse ses amis (passage des lignes) : les
  charges ne sont jamais freinées ni déviées.
- **Formations de l'IA** (`formation_ai.rs`, après `ai::plan`) : piquiers (`pike_square`) en schiltron
  quand de la cavalerie ennemie est à 120 m (retour en ligne au-delà de 220 m) ; lances
  (`charge_lance`) en coin pour une charge « propre » à moins de 250 m (pas de pieux ni de piques à
  100 m de la cible), gardé pendant la mêlée ; colonne pour une marche de plus de 250 m sans ennemi à
  300 m, redéploiement en ligne à 220 m. Coin : front de combat = 3 × files (au lieu de 2) ; correctif
  du placement des cavaliers dans le coin (débordement). **Flanc coordonné** : deux régiments montés qui
  contournent le même ennemi prennent chacun un flanc.
- **Déploiement** (`sim/deployment.rs`) — API pour le HUD :
  - `BattleSim.begin_deployment() -> bool` juste après `setup` (avant tout `tick`) : le temps est gelé
    (`tick` sans effet), seuls les ordres `formation` et `fire_at_will` passent, le camp IA est placé
    par rôles (ligne au centre, tireurs devant, ailes montées, engins derrière, général derrière le
    centre ; un camp nettement plus faible recule sur la meilleure hauteur de sa zone).
  - `is_deploying() -> bool`.
  - `get_deployment_zone(side) -> {x0, z0, x1, z1}` (mètres du champ). Bataille rangée : bande de
    300 m de profondeur le long de son bord (assaillant z ∈ [20, 300], défenseur z ∈ [500, 780]).
  - `deploy_unit(id, x, z, facing) -> {ok, error}` : `facing` en radians, `NAN` garde l'orientation ;
    refusé hors zone (« Les Chevaliers doivent être placés dans votre zone de déploiement » depuis F5d), pour une unité
    de l'autre camp, ou hors de la phase.
  - `start_battle() -> {ok, error}` : fin de la phase, la bataille commence au tick suivant.
  - Pendant la phase, `issue_command` renvoie « déploiement en cours… » pour les autres ordres.
- Point 5 (renforts au-delà de 20 régiments par camp) : fait en F5d (ci-dessous).
- Tests : `sim-battle/tests/f5.rs` (8 + 1 ignoré). Sonde `probe -- ai` : 2 × 20 régiments, assaillant
  4/10 (471 s en moyenne) ; 2 × 10, 6/10.
- **F5c — déploiement dans Godot** (`deployment_controller.gd`, `deployment_zone.gd`) : toute bataille du
  joueur (champ et siège) ouvre la phase après `setup` (`begin_deployment`). Zone du joueur au sol
  (remplissage doré translucide qui épouse le relief, liseré sur le contour, matériau non éclairé),
  bandeau « Déploiement — placez vos troupes » avec « Commencer la bataille » (ou **Entrée**) →
  `start_battle`. Clic droit : la sélection se range autour du point, orientation gardée ; glisser-droit :
  répartie sur la ligne, front tourné à l'opposé de la caméra (`deploy_unit` par régiment). Un refus
  s'affiche en message éphémère (« Chevaliers : l'unité 0 doit être placée… »). `--autoplay`,
  `--screenshot` saute la phase ; `--deploy-shot` (avec `--screenshot=`) la capture :
  `docs/img/godot-battle-deploy.png`. Sièges : maisons posées sur `get_siege().houses` (emprise inscrite
  dans chaque disque, l'église sur le disque le plus au fond), sortie de la garnison en message éphémère
  et dans la ligne de siège ; capture `docs/img/godot-siege-f5.png`. Smoke : phase ouverte, un placement
  valide et un refusé, `start_battle`, le temps avance ; maisons rendues = maisons de la simulation.

## F5d — correctifs de la simulation (Rust)
- **Bataille de démonstration** (`battle.tscn -- --screenshot=…`, France 1337, graine 1337, fixture
  `sim-battle/tests/fixtures/demo_battle_1337.json`) : plus de contact en 300 s. Cause : l'Angleterre,
  plus faible, reste sur la défensive (patience 480 s) ; la France, dont les arbalétriers « gagnaient »
  le duel de tir, gardait sa ligne là où elle se trouvait jusqu'à `DUEL_TIME` (480 s) — c'est-à-dire en
  pleine rivière, où elle s'était arrêtée à mi-traversée. Le défaut existait déjà avant F5a.
- Correctifs (`ai.rs`) :
  - un assaillant nettement plus fort (rapport ≥ 1/0,85, l'ennemi l'attend) abrège le duel à
    `ATTACKER_DUEL_TIME` = 60 s et, pour ouvrir le combat (personne encore au contact, avant 240 s,
    ennemi à moins de `ASSAULT_RANGE` = 250 m), lance sa cavalerie sur la cavalerie adverse ;
  - toute destination IA en eau profonde (hors gués) glisse sur la rive (`dry_z`, marge 12 m) : celle
    où se tient le régiment, ou l'autre s'il est déjà dans l'eau ; le déploiement IA
    (`begin_deployment`) applique la même règle ;
  - une ligne qui avance marche vers le barycentre ennemi quand celui-ci n'est plus devant elle (des
    armées qui se croisent ne filent plus vers le bord opposé).
- Mesures : démo — contact à 75 s (jamais avant), victoire française à 406 s ; `probe -- ai`
  (2 × 20) : assaillant 4/10, 468 s (avant : 4/10, 471 s) ; `probe -- ai 30` : 3/10, 690 s.
- **Renforts échelonnés** (`sim/reinforcements.rs`) : au-delà de `MAX_ON_FIELD` = 20 régiments par camp,
  les derniers (ordre des ids, jamais celui du général) attendent hors du champ (`Unit::reserve`, non
  `present`). Dès qu'un camp compte moins de 20 régiments combattants (détruit, en déroute, retiré), le
  premier en attente entre (déterministe) : bataille rangée par le bord du camp (z = 15 m ou
  profondeur − 15 m, cinq couloirs de 110 m), marche vers sa ligne ; garnison de siège depuis la place.
  Journal « Renforts : les … entrent sur le champ de bataille. » ; `strength` compte les réserves.
  Pont : `get_units()[].reserve` (`present` faux tant qu'il attend).
- `deploy_unit` : erreurs au nom du régiment (`BattleSim::error_text`), p. ex. « Les Chevaliers
  doivent être placés dans votre zone de déploiement ».
- Tests `sim-battle/tests/f5d.rs` : contact avant 150 s, personne déployé ni arrêté dans la rivière,
  rive choisie par `dry_z`, réserves et entrée par le bord, escalade ≈ 3/6 (voir `m8-sieges.md`).
