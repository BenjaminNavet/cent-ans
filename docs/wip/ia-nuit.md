# IA — nuit du 2026-09-30 : IA performante en campagne et en bataille

Branche `feat/ia` (worktree `../gp-ia`, depuis `main` 6a0d960c9). Mandat du joueur : « tu
travailles sur l'IA du jeu et tu t'assures qu'elle soit performante en campagne ou en bataille »,
autonomie toute la nuit.

« Performante » = les deux sens : l'IA joue bien (bataille : bat une IA naïve à forces égales,
exploite terrain et armes ; campagne : défend, prend les villes vides, ne se ruine pas) et elle
est rapide (tour de campagne < 50 ms par faction, spec M3 ; coût de l'IA par tick de bataille).

## Outils ajoutés
- `core/crates/sim-battle/tests/ia_survey.rs` (ignoré) : `survey` = IA contre un novice (chaque
  régiment attaque l'ennemi le plus proche toutes les 2 s), contre un camp passif, contre elle-même ;
  armées miroir / anglaise / française ; 4 terrains ; `IA_SELF=naive` = le camp mesuré joué en
  novice (référence). `plan_cost` = coût de `ai::plan` seul.
  `cargo test --release -p sim-battle --test ia_survey -- --ignored --nocapture survey`
  (16 graines ≈ 6 min).
- `core/crates/ai/examples/turn_hotspot.rs <seed> <tour> <faction> [reps]` : rejoue jusqu'au
  tour, chronomètre la planification d'une faction (`HOTSPOT_WAIT=1` pour `sample <pid>`).
- `core/crates/ai/examples/ia_quality_probe.rs` (agent) : qualité de l'IA de campagne.

## Mesures de départ (main 6a0d960c9)
- Coût bataille : `ai::plan` des deux camps 0,03 / 0,23 / 0,48 ms à 20 / 63 / 105 régiments par
  camp, une fois toutes les 2 s : négligeable.
- Coût campagne (`turn_perf --sequential 10 1 1`, machine chargée) : moyenne 5,1 ms, p99 22 ms,
  **pire 548 ms (Venise/Vérone t3)** → premier `ambush_orders` décode la carte de couverture
  (`cover_class_at`, OnceLock, ~1,2 s). Corrigé : préchauffage des rasters dans un fil au
  chargement des données (`godot-bridge/src/campaign_sim.rs load_shared_data`).
- Bataille, 16 graines × 4 terrains × 2 camps (victoires IA / 64) :

| adversaire | armées | IA attaque | IA défend |
|---|---|---|---|
| novice | miroir | 61 | 52 |
| novice | IA anglaise contre fr. | 61 | 63 |
| novice | IA française contre angl. | 38 | 48 |
| passif | miroir | 50 | 61 |
| passif | IA française contre angl. | 5 | 51 |
| IA | miroir | 30 | 34 |
| IA | IA française contre angl. | 2 | 19 |

  Référence novice contre novice : française contre anglaise 2/64 → l'IA apporte beaucoup.
  L'armée anglaise (5 000) bat la française (6 100) presque toujours : équilibre des unités
  (arc long), pas l'IA.
- Défaut vu (trace `IA_LOG=1`) : les chevaliers français attendent l'infanterie (EQ7) à leur poste
  d'aile, calé sur l'ancre d'avance de la ligne (30 m devant l'infanterie), à portée des arcs
  longs : 60 % de pertes sans frapper, puis charge épuisée et déroute.

## Essais
- w1 : poste d'aile jamais devant l'infanterie + recul hors de portée des tireurs tant que
  l'infanterie n'est pas en mêlée. Novice fr. attaque 38 → 51 ; mais passif miroir attaque 50 → 30,
  passif fr. défend 51 → 31 (la cavalerie arrive trop tard). Variantes en cours (`IA_WING`).

- Variantes du poste d'aile (novice + passif, /768) : base 618 ; plafond « jamais devant
  l'infanterie » partout 666 (mais 1er contact de la démo B6 à 192 s au lieu de ~70 s) ; + levée à
  80 m de l'ennemi 672 ; **retenu : seulement sous les flèches (pertes par tir < 40 s) + levée à
  80 m : 670, miroir inchangé, démo toujours ~70 s** (commit 92ec4dc18). EQ7 sonde 64 graines :
  Français 62/64 (avant 57/64). Empreintes B6 et bornes de rejeu (tick 704) mises à jour.
- Essai rejeté : tireurs IA visant la cible la plus rentable (armure, couvert) : 637/768 (les arcs
  anglais délaissaient les chevaliers) ; restreint au contre-tir sur les tireurs ennemis tant
  qu'aucune mêlée n'est à < 120 m : 673/768 (+3, bruit) → abandonné (KISS).
- Sièges tactiques non touchés : une autre session modifie `sim/siege_assault.rs` sur main.

## Campagne
- Sonde `ia_quality_probe` (agent), base 60 tours graines 1-4 : prises manquées 75, pertes sans
  secours 5,2, % oisives en guerre 22, sièges lancés 47 / pris 21,5 / **abandonnés 22,8**.
- Fin des sièges abandonnés (graines 1-2, 73) : **bataille contre une armée de secours au 1er tour
  (31)**, affamée/brisée 14, paix 15, défense/retraite 4, reciblage 2. L'abandon C7a ne joue jamais.
- Essai c1 (non commité) : défendre d'abord une place assiégée (×3) + pas de siège d'une place
  qu'une armée ennemie plus forte couvre (`threat_at`) → graines 1-4 : plutôt pire, très bruité ;
  comparaison sur 8 graines de plus en cours.

- c1 sur 12 graines (sonde globale) : sièges 43 → 53, pris 19,7 → 23,1, pertes sans secours
  4,9 → 5,7 : bruit ; la sonde globale ne tranche pas.
- **Nouvel outil : duel A/B** (`ai::experiment` + `examples/ai_duel_probe.rs <règles> <tours>
  <factions> <graines>`) : une faction joue la règle à l'essai, les autres les règles actuelles ;
  on compare son sort (places pondérées, provinces, puissance, trésor) à la même partie jouée sans
  essai. Déterministe (A = B sans règle). 8 factions (France, Angleterre, Écosse, Castille, Empire,
  Hongrie, Venise, Flandre) × graines 1-4, 60 tours.
- Résultats (mieux / pire sur 32, delta moyen des places) : `defend` (place assiégée ×3) 5/10,
  −7,6 ; `guard` (pas de siège sous une armée plus forte à 1 arête) 10/11, −1,9 ; les deux 9/13,
  −1,3 → **rejetées**.

- `reach2` (armées ennemies à 2 arêtes) 5/14, −9,1 ; `supply` (pas de siège sous 50 de vivres)
  5/8, −1,4 ; `siege12` (supériorité de siège 1,2 au lieu de 1,5) 8/8, +0,7 ; `assault50` (assaut
  dès 50 % au lieu de 65) 10/7, +0,65 ; `attack125` (attaque à 1,25:1 au lieu de 1,5) 4/8, −1,7 ;
  `warshare` (85 % du budget à l'armée en guerre au lieu de 70) 8/9, −1,3 → **aucune règle de
  campagne ne gagne son duel ; réglages inchangés** (ADR 0148). Portes d'essai retirées ; l'outil
  `ai::experiment` reste (inactif par défaut).
- Coût campagne après préchauffage (même préchauffage que le jeu, `turn_perf --sequential 20 1
  1 2`, machine chargée à 70-127) : moyenne 3,6 ms, p99 19 ms, pire 41 ms (Empire) < 50 ms.

## État : TERMINÉ sur la branche ; reste la fusion dans main

## Points ouverts
- Défenseur IA contre le novice en bocage / montagne (11/16) : ses archers perdent le duel
  d'archerie ; le contre-tir n'a rien donné.
- Sièges de campagne abandonnés au 1er tour (armée de secours) : les parades essayées (`guard`,
  `reach2`) font perdre la faction qui les joue ; à revoir côté règle (levée de siège ?) plutôt
  qu'IA.
- Petits royaumes (≤ 2 provinces) jamais assiégés sans prétention (F4, voulu) : des armées
  restent oisives devant eux pendant des guerres longues.
- Sièges tactiques non mesurés (une autre session modifie `siege_assault.rs`).
