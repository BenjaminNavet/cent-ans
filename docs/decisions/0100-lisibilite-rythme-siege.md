# 0100 — Lisibilité et rythme de la destruction en siège (lot SB)

Date : 2026-09-28. Statut : accepté. Spec : `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § SB.

## Contexte

En bataille de siège, les PV des pièces (`get_siege().pieces[i].hp/max_hp`) existaient mais
n'étaient pas montrés : `battle_siege.gd` ne faisait que noircir et abaisser les pans. Le rythme
était trop lent pour une bataille façon Total War : au niveau 3, porte ≈ 1 min 50 au bélier, brèche
≈ 7 min 20 au trébuchet (37 tirs). Aux niveaux 4-5, un trébuchet seul (40 pierres) ou une bombarde
(20 boulets) ne pouvaient même plus ouvrir un pan.

## Décision

### Cœur (sim-battle, pont)
- `WallPiece.attacked_for` (s) et `WallPiece::under_attack()` : une pièce est **visée** tant qu'un
  bélier est au contact de la porte ou qu'elle brûle (marque de `UNDER_ATTACK_CONTACT_S` = 4 s,
  rafraîchie chaque pas), ou pendant `UNDER_ATTACK_SHOT_S` = 15 s après un tir d'engin (un peu plus
  que le rechargement de 12 s : un pilonnage continu se lit comme une seule attaque). La marque
  décroît dans `resolve_siege_works`. Le cœur décide, Godot n'a rien à deviner.
- `BattleSim::siege_engines()` : béliers et beffrois présents, avec leur force (`hp` = servants
  restants, `max_hp` = équipage complet) et leur camp.
- `get_siege()` expose `pieces[i].under_attack` et `engines[{unit, kind: "ram"|"tower", side, x, z,
  hp, max_hp}]`.

### Données (`data/rules/siege_works.json`, schéma inchangé)
- Mur : 1300 + 600/niveau → **100 + 200/niveau**. Porte : 300 + 80/niveau → **40 + 70/niveau**.
  Bélier (5 PV/s) et engins (1,2 PV par point de `siege_attack`) inchangés : les engins gardent
  leur hiérarchie (bombarde 85 > trébuchet 70 > mangonneau 40).

### Rendu (Godot)
- `game/scripts/battle/siege_health_bars.gd` (`SiegeHealthBars`, `CanvasLayer`) : petite barre
  enluminée (vélin, filet d'encre et d'or, quarts gradués, remplissage à la couleur du camp
  propriétaire tirée de `battle_scene.side_colors`) au-dessus de chaque pièce endommagée ou visée,
  et de chaque bélier/beffroi entamé ; masquée si intacte et non visée, ou tombée (les gravats
  suffisent). Contrôles 2D placés par projection de la caméra : taille constante à l'écran.
  Étiquette « Porte : 324/540 » au-dessus des pièces visées et au survol ; les barres ne captent
  aucun clic (survol calculé à la main).
- `BattleSiege` crée les barres dans `build` et les met à jour dans `update` (lecture unique de
  `get_siege` par image, déjà en place).

## Mesures

Sonde `core/crates/sim-battle/tests/sb_siege_pace.rs` (`cargo test --release -p sim-battle --test
sb_siege_pace -- --ignored --nocapture`) : bélier à plein équipage contre la porte non défendue ;
un engin seul à 150 m contre un pan de front (ordre `TargetWall`). Les tours tirent sur l'engin :
la sonde maintient l'équipage et le moral pleins (la spec vise le rythme d'un équipage complet).
« — » : pan non tombé en 15 min (munitions épuisées).

Avant :

| Niveau | PV porte | Bélier | PV pan | Trébuchet | Bombarde | Mangonneau |
|---|---|---|---|---|---|---|
| 0 | 300 | 1:00 | 1300 | 3:03 (16 tirs) | 2:26 (13 tirs) | 5:30 (28 tirs) |
| 1 | 380 | 1:18 | 1900 | 4:28 (23 tirs) | 3:40 (19 tirs) | 7:56 (40 tirs) |
| 2 | 460 | 1:33 | 2500 | 5:54 (30 tirs) | — (20 tirs) | — (40 tirs) |
| 3 | 540 | 1:48 | 3100 | 7:19 (37 tirs) | — (20 tirs) | — (40 tirs) |
| 4 | 620 | 2:06 | 3700 | — (40 tirs) | — (20 tirs) | — (40 tirs) |
| 5 | 700 | 2:21 | 4300 | — (40 tirs) | — (20 tirs) | — (40 tirs) |

Après :

| Niveau | PV porte | Bélier | PV pan | Trébuchet | Bombarde | Mangonneau |
|---|---|---|---|---|---|---|
| 0 | 40 | 0:09 | 100 | 0:12 (2 tirs) | 0:00 (1 tir) | 0:25 (3 tirs) |
| 1 | 110 | 0:24 | 300 | 0:37 (4 tirs) | 0:25 (3 tirs) | 1:13 (7 tirs) |
| 2 | 180 | 0:36 | 500 | 1:01 (6 tirs) | 0:49 (5 tirs) | 2:02 (11 tirs) |
| 3 | 250 | 0:51 | 700 | 1:38 (9 tirs) | 1:13 (7 tirs) | 2:51 (15 tirs) |
| 4 | 320 | 1:06 | 900 | 2:02 (11 tirs) | 1:38 (9 tirs) | 3:40 (19 tirs) |
| 5 | 390 | 1:18 | 1100 | 2:39 (14 tirs) | 2:02 (11 tirs) | 4:28 (23 tirs) |

Niveau 5 / niveau 3 : porte ×1,53, pan ×1,56 (tirs). Tests : `level_three_meets_the_pace_targets`,
`stronger_towns_hold_longer`, `pieces_under_attack_are_flagged`, `siege_engines_report_their_strength`.

### Équilibre des prises (IA contre IA)

`sg3_assault_probe` (armées de campagne, villes emblématiques, 10 graines, 600 s) : **taux de prise
inchangé**, porte plus souvent tombée, durées voisines.

| Ville | Porte tombée avant → après | Victoires assaillant avant → après | Durée médiane avant → après |
|---|---|---|---|
| Paris | 4/10 → 10/10 | 10/10 → 10/10 | 271 → 291 s |
| Avignon | 2/10 → 10/10 | 10/10 → 10/10 | 255 → 237 s |
| Bruges | 0/10 → 10/10 | 10/10 → 10/10 | 206 → 178 s |
| Calais | 3/10 → 8/10 | 10/10 → 10/10 | 236 → 211 s |
| Rouen | 3/10 → 10/10 | 7/10 → 7/10 | 347 → 347 s |

`br3_assault_probe` (banc BR3 : 11 régiments dont trébuchet, mangonneau, beffroi contre 6 de
garnison, niveau 2, murs entamés à 40 %) : **bascule** de 3/10, 3/10, 2/10 (générique, Paris,
Rouen) à 10/10 partout ; durée médiane 391 → 201 s (générique). La trace montre pourquoi : avant, la
première ouverture venait vers 275 s et l'assaillant, massé au pied des murs sous le tir des tours,
comptait ≈ 11 déroutes par partie perdue ; il ouvre maintenant vers 70 s et entre avant d'être usé.
Ce banc oppose presque deux contre un : la victoire de l'assaillant y est plausible, et les
défaites d'avant étaient l'effet du rythme lent que SB corrige. Aucune compensation n'est prise ici
(elle toucherait le moral ou les tours, hors du lot) : le « dernier carré » des défenseurs sur la
place (lot T4) est le levier prévu si les sièges joués se révèlent trop faciles. L'auto-résolution
de campagne n'utilise pas ces PV (aucun effet).

## Conséquences

- Les sièges se jouent en quelques minutes : l'enjeu passe des murs à la mêlée dans la brèche et
  à la place centrale (préparation de T4, points de capture).
- La porte en feu (`siege_fire.json`, 4 PV/s × intensité) tombe aussi plus vite, dans la même
  proportion que sous le bélier (≈ 1 min au niveau 3 au lieu de ≈ 2 min 15) : laissé tel quel.
- L'invariant de `sg3_assault_probe` « un pan tient au moins 3× plus que la porte » devient
  « au moins 1,5× » (niveau 3 : 51 s contre 1 min 48).
- Les tours du rempart tirent toujours sur les engins et les béliers ; en vraie bataille les temps
  sont donc plus longs que ceux du tableau (équipage entamé, relève SG4).
