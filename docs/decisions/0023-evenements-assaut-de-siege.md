# ADR 0023 — Assaut de siège : événements de rendu du cœur et grimpeurs posés par le cœur

Date : 2026-09-25. Statut : accepté. Lot SG1 (backlog `docs/audit/backlog-tw.md`, « Sièges »).

## Contexte

Les batailles de siège (M8, F5, S1, S2) avaient leurs règles dans `sim-battle`, mais le rendu les
devinait en comparant l'état d'une image à l'autre : PV d'un pan qui baissent (son d'impact),
munitions qui baissent (volée tirée… vers le régiment ennemi le plus proche, même quand l'engin
battait la muraille), un régiment `climbing` entier élevé d'un bloc à `climb_progress × hauteur`.
Impossible ainsi de montrer la pierre qui frappe tel endroit du pan, le coup de bélier, les
échelles dressées là où la simulation les pose, le pont-levis d'un beffroi qui accoste ou l'huile
versée des mâchicoulis.

## Décision

1. **Flux d'événements de rendu** distinct du journal : `SiegeFx { time, kind }`
   (`sim-battle/src/siege_fx.rs`), lu par `BattleSim.get_siege_events()` (comme `get_events()`,
   « depuis le dernier appel »). Le cœur applique d'abord ses règles puis enregistre ce qui s'est
   passé ; un rendu qui ignore ces événements ne perd que le spectacle. Aucun tirage aléatoire :
   le point d'impact d'une pierre vient d'un hachage entier (tick, unité), le déterminisme et les
   tests existants sont intacts. Les transitions (porte enfoncée, brèche, beffroi accosté ou
   décroché) sont détectées une fois par pas, quelle qu'en soit la cause (bélier, engin, feu).
2. **Grimpeurs posés par le cœur** : `BattleSim::soldier_poses(unit)` remplace
   `soldier_positions + standing_height` pour le tampon des soldats. Un régiment qui escalade
   montre ses premiers soldats sur les échelles (`BattleSim::ladders`, pied et haut sur le pan,
   à l'écart des tours) ou sur le pont du beffroi, montant homme après homme, une part croissante
   des autres déjà sur le chemin de ronde, le reste au pied du mur. Le rendu pose ses échelles sur
   les mêmes segments (`get_units().ladder_lines`) : pas de géométrie dupliquée.
3. **Animation d'escalade** : clip `climb` cuit par le pipeline V2 (surcouche de pose, ADR 0014)
   et mode de shader `M_SPLIT` : les `split_count` premiers soldats d'un régiment
   (`climbers_shown`) jouent le clip d'escalade, les autres les clips du pied du mur. Le tampon
   de la simulation reste copié tel quel dans le `MultiMesh` (aucun reconditionnement GDScript).
4. **Règles ajoutées avec** (dans le cœur, testées) : bélier par coups toutes les 3 s (même usure
   moyenne), huile bouillante (un pot toutes les 30 s si un défenseur garde la porte, 3 hommes
   avant armure à moitié efficace, −4 de moral ; bélier protégé par ses peaux), repli de la
   garnison IA sur la place quand la porte tombe (2 régiments au plus par ouverture la bouchent).
5. **Rendu** dans un fichier séparé (`game/scripts/battle/siege_assault_fx.gd`) : `battle_siege.gd`
   (maisons BR1, pans S1, feu S2) ne gagne qu'un drapeau (`external_ladders`) et le décalage
   visuel des tours de la porte (`SiegeAssaultFx.gatehouse_tower`).

## Options écartées

- Tout déduire des différences d'état côté GDScript : ni point d'impact, ni instant du coup, ni
  pan visé par un engin (la cible d'unité est vide quand il bat la muraille).
- Mettre ces événements dans le journal (`BattleEvent`) : il parle au joueur, en français ; le
  mélanger au rendu l'aurait pollué (une ligne par pierre).
- Un `MultiMesh` séparé pour les grimpeurs : exclusion côté cœur, second tampon, second matériau ;
  le mode `M_SPLIT` suffit avec un seul entier par régiment.

## Conséquences

- Nouveau spectacle de siège = un variant de `SiegeFxKind` + son traitement dans
  `siege_assault_fx.gd`.
- Sonde d'escalade (`SEEDS=20 probe -- siege 0 ""`) : 9/20 → 7/20 victoires de l'assaillant (huile),
  dans la fourchette de `f5d` ; brèche et engins inchangés (6/6).
- Les tours de la porte sont dessinées écartées de ≈ 0,15 rayon au-delà des jambages ; le cœur
  les garde centrées sur les extrémités (elles ne sont pas des obstacles, seul leur tir compte).

## Suite SG2 (2026-09-25) : engins animés d'après le cœur

- Le cœur expose `get_units().reload` / `reload_period` (`Unit::reload_period`,
  `shot::ENGINE_RELOAD` = 12 s, même valeur qu'avant) : le rendu ramène la verge du trébuchet au
  treuil au fil du rechargement et commence le basculement `swing_s × release_phase` secondes
  avant la fin du rechargement quand le régiment tire, pour que la fronde lâche à l'instant du
  `ShotEvent` ; si le tir n'était pas prévu, le basculement part au tir et la pierre part avec un
  léger retard, vol raccourci d'autant (l'impact reste proche des dégâts du cœur).
- Modèles Blender à pièces nommées (`tools/blender_scripts/siege_engines.py`,
  `game/assets/models/siege/`), animés par nœuds (`SiegeEnginesFx`), aucun clip cuit ; réglages
  dans `data/fx/siege_engines.json` (schéma `siege_engines.schema.json`). Les figurines d'engin
  de `BattleMeshes` ne gardent que leurs servants quand le modèle animé existe.
- Démo : `CampaignState::debug_stage_landmark_siege` assiège la ville d'un plan (Avignon,
  Bruges) en déclarant la guerre à son détenteur (campagne jetable) ; menu « Batailles de
  démonstration » (`data/ui/battle_demos.json`).

## Suite SG3 (2026-09-25) : résistance des ouvrages en données, servants, LOD

- Les PV des murs et de la porte et les dégâts du bélier et des engins quittent le code pour
  `data/rules/siege_works.json` (schéma `siege_works_rules.schema.json`, `SiegeWorkRules`,
  embarqué à la compilation comme `siege_fire.json`). Réglage : porte en bois 300 + 80 × fort.,
  bélier 5 PV/s ; mur 1300 + 600 × fort., tir d'engin 1,2 × siege_attack : au niveau 5 la porte
  cède en 140 s de pilonnage, un pan sous un trébuchet seul en ~10 min. Sonde
  `sim-campaign/tests/sg3_assault_probe.rs` (5 villes × 10 graines, IA des deux côtés).
- Servants d'engins : figurines skinnées V2 dédiées (`crew_0`, `crew_1` au refouloir) et clips
  `crank`, `haul`, `load`, `swab`, `push` ; `SiegeCrewFx` (un MultiMesh par figurine, clip et
  camp, mode CUSTOM du shader) choisit le geste d'après `reload` / `reload_period` du cœur ;
  placement et rôles dans `data/fx/siege_engines.json` (`crew`). Rendu seulement.
- LOD : `<modèle>_lod.glb` (mêmes pièces nommées, petites pièces retirées) au-delà de
  `lod.simple_m`, pose ralentie et servants cachés au-delà de `lod.far_m` (= distance des
  imposteurs BV3) ; `SiegeEnginesFx.lod_distances()` pour les préréglages de qualité.
