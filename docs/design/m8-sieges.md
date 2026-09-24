# M8 — Sièges : spécification

Date : 2026-09-23. Deux volets : la guerre de siège sur la carte de campagne, puis la bataille de
siège en 3D sur le moteur temps réel de M7. **État : terminé (les deux volets).**

## 1. Campagne (`sim-campaign/src/siege.rs`) — fait
- `SiegeState` gagne `turns_elapsed`, `supplies` (0-100, départ 100 − dévastation/2) et `breach` (0-100).
- Vivres : perte par tour = 100 / (2 + fortifications), raccourcie par le `SiegeSpeed` du général
  assiégeant ; à 0, la garnison affamée capitule (prise de la place). `turns_left` = estimation de la
  reddition, affichée.
- Brèche : Σ `siege_attack` des engins (trébuchet 70, mangonneau 40, bombarde 85) × effectif relatif /
  (2 × (1 + fortifications)) par tour ; dès 50, ou avec une tour de siège (`wall_assault`), l'assaut ne
  subit plus la pénalité des murailles (-30 %).
- Ordre `assault { army }` : bataille auto-résolue armée contre garnison (général : gouverneur), victoire =
  prise de la place (hook `on_siege_won`, score de guerre), défaite = pertes, le siège continue.
  `CampaignState::assault_odds(army)` estime les chances pour l'interface et l'IA (assaut si ≥ 65 %).
- Sortie : la garnison attaque si sa puissance dépasse 1,3 × celle des assiégeants ; victoire = siège levé.
- Pont : `get_province_state().siege` gagne `turns_elapsed`, `supplies`, `breach` ;
  `get_assault_odds(army)`, `get_pending_events()`. Interface : ligne de siège et bouton « Donner l'assaut
  (chances ≈ x %) » dans le panneau d'armée (`scripts/map/siege_controller.gd`). Capture
  `docs/img/godot-siege.png` (`--stage=siege`). 7 tests (`tests/m8.rs`).

## 2. Bataille de siège 3D — fait
- `sim-battle` : carte de siège (enceinte polygonale avec tours et porte, épaisseur selon les fortifications,
  brèches existantes reportées depuis `breach`), défenseurs sur le chemin de ronde (bonus de tir et de
  défense), échelles (infanterie, lent, vulnérable), tours de siège (portent l'infanterie sur le rempart),
  bélier contre la porte, engins qui tirent sur les murs pendant la bataille, défenseurs qui abandonnent le
  rempart en déroute ; victoire de l'assaillant = contrôle de la place centrale pendant 60 s ou défenseurs
  en déroute.
- L'ordre `assault` du joueur, si `interactive_battles`, crée une bataille en attente de type siège
  (même flux que M7 : dialogue « Livrer l'assaut / Résolution automatique »).
- Godot : murailles, tours, porte low-poly procédurales (ou modèles Blender de M10), échelles et tours de
  siège animées, défenseurs positionnés sur les remparts.
- Tests : une brèche large facilite l'assaut, les échelles sans brèche sont coûteuses, le bélier ouvre la porte.

### Réalisation (`core/crates/sim-battle/src/siege.rs`, `sim.rs`, `ai.rs`)
- `BattleSetup.siege: Option<SiegeSetup { fortification, breach }>` (serde par défaut : bataille rangée).
  `SiegeWorks::generate` : octogone irrégulier de rayon ≈ 150 m centré en (600, 560), le côté sud (face à
  l'assaillant) coupé en deux courtines et une porte de 14 m ; tours rondes aux angles et de part et
  d'autre de la porte ; place centrale de 35 m. Épaisseur 2,5 + 0,5 × fortif. m, hauteur 6 + 1,5 × fortif. m,
  PV d'un pan 500 × (1 + fortif.), porte 250 × (1 + fortif.). Brèche de campagne : les pans perdent jusqu'à
  40 % de PV, un pan de façade est ouvert dès 50, deux dès 85. Le champ est aplani autour de la ville,
  sans rivière ni forêt sur l'approche.
- Déploiement : assiégeants hors de portée d'arc (infanterie à 265 m de la façade, tireurs à 215 m,
  engins à 240 m), tours de siège et bélier plus près ; garnison sur le chemin de ronde face à
  l'extérieur (tireurs d'abord, façade puis flancs), une garde derrière la porte, une réserve (⅓ des
  fantassins) et la cavalerie sur la place. Les chevaliers et hommes d'armes montés de l'assaillant
  (`dismount`) mettent pied à terre pour l'assaut. Chaque assiégeant reçoit un **bélier** (unité de
  bataille « synthétique », 12 servants, non rapportée à la campagne).
- Murailles : un pan intact arrête les régiments (bande de ½ épaisseur + 1,5 m), le corps à corps et la
  vue à travers lui ; seuls les défenseurs peuvent monter sur le chemin de ronde depuis l'intérieur ;
  les régiments en déroute passent (poternes). Un pan à 0 PV (brèche, porte enfoncée) est une ouverture :
  les ordres de mouvement la contournent automatiquement (point d'approche puis traversée).
- Chemin de ronde : précision +25 %, portée augmentée par la hauteur, tirs reçus ×0,5 (merlons),
  pertes de moral ×0,7 et moral qui remonte même au contact ; en déroute, le régiment abandonne le
  rempart (« Les … abandonnent le rempart ! »).
- Échelles : un fantassin de l'assaillant arrêté par un pan l'escalade (état `climbing`, 45 s,
  ×0,25 au contact) ; sur l'échelle il frappe ×0,3 et reçoit ×2 au corps à corps, ×1,5 aux traits. Une
  tour de siège (`wall_assault`) accostée à un pan (elle roule à 0,55 m/s au moins, couverte : traits
  ×0,1) fait passer l'infanterie proche en 8 s sans pénalité. Au sommet, le régiment prend pied sur le
  rempart et combat les défenseurs.
- Bélier : au contact de la porte, 4 PV/s × servants restants ; « La porte cède sous les coups du
  bélier ! ». Engins (`siege_attack`) : sans cible d'unité, ils battent le pan le plus proche à portée
  (ou celui de la commande `target_wall { units, piece }`) : 1,6 × `siege_attack` × servants par tir,
  toutes les 12 s ; le pan effondré fait tomber ses défenseurs (12 % de pertes, −10 de moral).
- Victoire de l'assaillant : garnison en déroute, ou place centrale tenue 60 s (au moins un régiment
  assaillant apte dedans, aucun défenseur apte ; le compteur redescend de moitié moins vite quand la
  place est disputée). Sinon la nuit (1 h) donne la victoire au défenseur ; l'IA assaillante sonne la
  retraite quand plus personne ne peut entrer.
- Campagne (`siege.rs`, `battle_request.rs`) : avec `interactive_battles`, l'ordre `assault` du joueur —
  ou d'une IA contre une place du joueur — crée une `BattleRequest { siege: true }` (champ
  `#[serde(default)]`, `STATE_VERSION` inchangé = 4 ; `defender` reprend l'id de l'assaillant, le
  défenseur est la garnison, général = gouverneur). `battle_setup` fournit la garnison et
  `SiegeSetup { fortification_level, breach }` ; `resolve_pending_battle` applique les pertes aux deux
  camps et, en cas de victoire, la prise de la place (`capture`, score de guerre, `on_siege_won`) comme
  l'assaut automatique ; `auto_resolve_pending` et le début du tour suivant utilisent l'assaut automatique.
  `debug_stage_siege(army, province)` pour les tests et captures.
- Pont : `get_terrain().siege`, `BattleSim.get_siege()` (pans avec PV, tours, porte, place, tenue),
  `get_units()` gagne `on_wall`, `climbing`, `climb_progress`, `ladders`, `synthetic`, `ram`,
  `siege_tower`, `wall_breaker` (et `render` = `ram` / `tower`) ; `get_pending_battles()` gagne `siege`,
  `fortification`, `breach` ; `CampaignSim.debug_stage_siege`.
- Godot : `scripts/battle/battle_siege.gd` (courtines crénelées, tours à toit conique, porte à vantaux,
  place pavée, maisons, cathédrale M10 si `assets/models/cathedral.glb` existe ; pans assombris,
  abaissés puis effondrés en éboulis, porte enfoncée ; beffrois, bélier, échelles des régiments qui
  escaladent), ligne d'état du siège dans le HUD, dialogue « Assaut en vue / Livrer l'assaut », bouton
  « Donner l'assaut » qui ouvre ce dialogue. Capture : `docs/img/godot-siege-battle.png`
  (`battle.tscn -- --siege --screenshot=…`).
- Tests : 10 dans `sim-battle/tests/siege.rs` (enceinte et porte, brèches de campagne, murs qui
  arrêtent puis brèche franchie, échelles lentes et tour plus rapide, bélier, engins et pan effondré,
  place tenue 60 s, défenseurs qui abandonnent le rempart, brèche large vs échelles sur 4 assauts IA,
  déterminisme) ; 3 dans `sim-campaign/tests/m8_battle.rs` (assaut en attente, victoire 3D = prise,
  résolution auto et sauvegarde) ; smoke § 11. Sonde : `cargo run --release -p sim-battle --example
  probe -- siege <brèche> [engins,…]` (garnison de 5 contre 8 régiments + engins : échelles seules
  ≈ 3/6 victoires, brèche ouverte 6/6, 3-6 min).

## F5 — finition des sièges (F5a, Rust)
- **Déploiement** : même API que la bataille rangée (`m7-battles.md` § F5). Zone de l'assaillant :
  tout le champ au sud de l'enceinte, jusqu'à 60 m de sa face la plus avancée (`SIEGE_STANDOFF`),
  jamais dans l'enceinte ni sur la bande d'un pan intact ; zone de la garnison : rectangle englobant
  l'enceinte, point exigé **dans** l'anneau (un défenseur posé près d'un pan monte sur le chemin de
  ronde au premier tick). Tours de siège et bélier gardent leur place par défaut (plus près).
- **Maisons et rues** : `SiegeWorks.houses` (disques de 9 m, deux couronnes à 75 et 110 m du centre,
  rues rayonnantes de la place vers chaque pan et la porte ; génération sans tirage aléatoire).
  Exposées au pont : `get_siege().houses = [{x, z, radius}]` — **le rendu Godot doit poser ses
  maisons sur ces disques** (fait en F5c : `battle_siege.gd` lit `houses`). Les maisons arrêtent les
  régiments (sauf en déroute).
- **Cheminement A*** (`sim/pathing.rs`) : grille de 4 m sur tout le champ ; obstacles = bandes des
  pans intacts et maisons ; brèches, porte enfoncée (et porte ouverte d'une sortie, pour la garnison)
  libres. Chemin mis en cache par régiment, recalculé quand la cellule but ou les ouvertures changent ;
  lissage à vue ; un régiment collé à un jambage recule d'abord. Les grimpeurs gardent la règle M8
  (échelles si le détour dépasse 1,6 × + 40 m). La règle d'arrêt contre un pan compte désormais la
  distance au pan lui-même : on contourne librement le jambage d'une brèche.
- **Tours de l'enceinte** (`sim/siege_extra.rs`) : chaque tour encore reliée à un pan intact tire toutes
  les 8 s (décalées) sur l'assiégeant le plus proche hors des murs à 180 m (25 tireurs, précision
  0,3 → 0,15 avec la distance, météo), tant que la garnison a un régiment apte ; jamais sur le bélier
  ni les tours de siège.
- **Sortie** : garnison IA, après 2 min, quand la valeur des assiégeants tombe sous la moitié de la
  sienne → `SiegeWorks.sortie` (`get_siege().sortie`), « La garnison ouvre ses portes et fait une
  sortie ! » ; la porte laisse passer la garnison, fantassins et cavaliers (hors tireurs du rempart)
  chargent l'assiégeant le plus proche.
- Tests `f5.rs` : zones de siège, cheminement par la brèche en évitant les maisons, tours et sortie,
  déterminisme. Sonde `probe -- siege 0` : 6/6 (≈ 290 s) ; `siege 60` : 6/6 (≈ 210 s) — l'échelade
  sans brèche réussit plus souvent qu'en M8 (3/6) : l'infanterie trouve maintenant la porte enfoncée
  par le bélier. À rééquilibrer si besoin (PV de la porte, tir des tours).

## F5d — rééquilibrage de l'escalade (Rust)
- Constat : la comparaison F5a ci-dessus mélangeait deux sondes. `probe -- siege 0` (avec deux
  trébuchets et une tour) donnait déjà 6/6 en M8 : les trébuchets ouvrent deux brèches vers 120 et
  200 s, c'est un assaut par la brèche. L'escalade proprement dite (`probe -- siege 0 ""`, échelles et
  bélier seulement) faisait 3/6 en M8 et était tombée à 2/6 (4/20 sur 20 graines) avec le tir des
  tours de F5a ; le bélier, lui, ne pèse pas (il n'enfonce jamais la porte avant la décision :
  PV de la porte, dégâts et armure du bélier testés sans effet).
- Réglage : chaque tour tire 5 carreaux par salve au lieu de 25 (`TOWER_SHOTS`, portée 180 m et
  cadence 8 s inchangées) : les tours pèsent sans décider seules.
- Mesures : `siege 0 ""` → 3/6 (8/20 sur 20 graines, `SEEDS=20`) ; `siege 0` → 6/6 (≈ 290 s) ;
  `siege 60` → 6/6 (≈ 210 s). La sonde accepte désormais `SEEDS=n`. Test
  `f5d::a_ladder_escalade_wins_about_half_the_time` (2 à 4 victoires sur 6) ; tests de siège M8 et F5a
  verts. Garnison en réserve au-delà de 20 régiments : entre depuis la place (`m7-battles.md` § F5d).

## G1 — alliés dans l'assaut
`siege::assault_coalition(army)` : l'armée assiégeante puis les armées de sa faction ou de ses alliés dans
la province, en guerre avec le contrôleur de la place (même règle que `movement::battle_coalition`).
L'auto-résolution fait donner la coalition (`coalition_side`), les pertes sont réparties par régiment, le
général commandant est le meilleur par commandement ; `battle_setup` du siège exporte l'armée combinée et
`resolve_pending_battle` la valide dans le même ordre. L'assaut devient une bataille en attente dès que le
joueur y a une armée, même alliée. La garnison combat seule.

## S2 — incendies (spéc `s2-incendies.md`)
Les maisons (`SiegeWorks.houses`, plus 4 faubourgs hors les murs) et la porte peuvent brûler : traits
et pots à feu de l'assaillant, propagation selon la distance, le vent et la météo, chaleur et fumée,
ruines franchissables par l'A*. Pont : `get_siege().houses[i]` gagne `fire = {state, intensity}` et
`suburb` ; la racine gagne `gate_fire`, `wind`, `houses_burning`, `houses_burnt` ; commande
`{type: "burn", units, house | gate: true}` ; `debug_ignite(house)`. ADR `0008-incendies-regle-coeur.md`.
