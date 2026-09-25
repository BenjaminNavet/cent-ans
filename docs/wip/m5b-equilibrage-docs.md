# M5b — équilibrage du mouvement libre et documentation (état)

Spec : `docs/design/2026-09-24-mouvement-libre.md` § 7-8. Suivi : `docs/wip/mouvement-libre.md`.
Branche : `worktree-agent-a3c4f4bdfe00c9af3`, depuis main fa7efb3c (M1-M4).

## État

- [x] Mesure de référence (`settlements_probe 50 1..8`, release) : identique au M3 de `m3-tour-ia.md`.
- [x] Débandades : règle « refuge » + défaite lourde (voir Décisions).
- [x] Passages de fleuves : croisements Itiner-e gardés à moins de 6 km d'une colonie ou d'un pont/gué.
- [x] Saône en aval de Chalon : tronçon nommé « Sane » dans `rivers.geojson` (encodage cassé), rattaché.
- [x] Somme : grand fleuve, Blanchetaque + 6 ponts (Abbeville, Pont-Remy, Picquigny, Amiens, Corbie, Péronne).
- [x] Portée vérifiée (`examples/march_range_probe.rs`) : aucun réglage changé.
- [x] Écosse : pas liée au mouvement libre (voir plus bas).
- [x] Manuel (§ 4, § 5 « Mouvement libre des armées », « Repli »), encyclopédie (`mech_movement`, sièges), codex
  (`cdx_ponts_gues`, `cdx_zone_controle`, `cdx_deroute_debandade` mis à jour), tutoriel (étapes « marche »,
  « fin du tour », conseil anglais).
- [ ] Fusion de main, vérifications finales.

## Mesures (`settlements_probe 50 1..8`, release)

| Indicateur | avant M2 (C7a) | M3 (départ) | + débandades (ancienne grille) | **M5b final** (rayon 6 km) | variante rayon 10 km |
|---|---|---|---|---|---|
| Trésor final FR / EN | 96 240 / 18 326 | 110 978 / 14 450 | 110 630 / 16 042 | 87 103 / 12 950 | 104 430 / 15 072 |
| Δ colonies FR / EN | −1,8 / −0,9 | +0,2 / −2,8 | +0,1 / −2,5 | −1,8 / −1,1 | +0,6 / −4,0 |
| Écosse Δ colonies ; banqueroutes | −0,6 ; 8 | −1,2 ; 15 | −0,6 ; 18 | −1,9 ; 4 | −0,6 ; 7 |
| Sièges / tour (cités / autres) | 0,8 / 3,1 | 0,6 / 3,0 | 0,5 / 2,9 | 0,8 / 3,8 | 0,8 / 3,8 |
| Prises / graine (cités / secondaires) | 10,1 / 65,8 | 8,2 / 55,0 | 8,4 / 54,1 | 11,0 / 78,0 | 13,0 / 80,9 |
| Batailles / graine | 58,9 | 79,2 | 75,9 | 89,4 | 112,6 |
| Débarquements hostiles / graine | 3,5 | 3,4 | 3,1 | 3,1 | 3,9 |
| Armées bloquées / tour | 0,3 | 0,1 | 0,2 | 0,2 | 0,3 |
| Débandades / graine (dispersées) | 1,2 | 0 | 1,2 (0,9) | 1,8 (1,6) | 2,1 (1,1) |

Sur 604 replis (ancienne grille, 8 graines) : 585 vers une place amie, 10 reculs de 15 km, 9 débandades. Les
pertes du vaincu en bataille automatique plafonnent vers 45 % (médiane 39 %).

## Passages de fleuves

Croisements voie romaine / grand fleuve (composantes hors ponts de `crossings.json`), avant → après :
Rhin 46 → 13, Danube 42 → 4, Meuse 41 → 7, Tage 40 → 1, Rhône 39 → 3, Loire 31 → 9, Èbre 26 → 9,
Seine 24 → 5, Garonne 20 → 7, Pô 18 → 2, Dordogne 12 → 2, Escaut 9 → 4, Saône 9 → 3, Tamise 5 → 5,
Somme — → 3. Total 362 → 77 (290 écartés). S'y ajoutent 102 ponts et gués appliqués (91 avant : +6 Somme,
+Blanchetaque, +4 ponts de la Saône aval désormais sur le tracé) et les colonies bâties sur un fleuve. La
Loire compte ainsi 13 ponts + 9 croisements. Grille connexe (contrôles M1), 97,8 % de la terre franchissable.

Effet : moins de raccourcis, les armées se concentrent sur les ponts, qui sont des places : plus de sièges
et de prises (retour vers les chiffres C7a pour les cités), plus de batailles. Le rayon de 10 km donne
davantage de batailles et fait reculer l'Angleterre (−4 colonies) ; 6 km retenu.

## Portée (été : 1 460 points = 210 km de plaine ; hiver 974 = 140 km)

| Paris → | vol d'oiseau | chemin (km de plaine) | tours été | tours hiver |
|---|---|---|---|---|
| Orléans | 111 | 97 | 1 | 1 |
| Rouen / Reims | 112 / 131 | 97 / 121 | 1 | 1 |
| Tours | 204 | 191 | 1 | 2 |
| Calais | 236 | 219 | 2 | 2 |
| Lyon | 392 | 408 | 2 | 3 |
| Bordeaux | 499 | 467 | 3 | 4 |
| Toulouse / Bayonne | 587 / 664 | 618 / 615 | 3 | 5 |

Conforme au jeu voulu (Orléans ≈ 1 tour, Bordeaux ≈ 3) : rien changé.

## Décisions

- Débandades (`movement.rs::retreat_target_after`) : après l'échec du repli sur une place amie, le recul de
  15 km n'est permis que s'il reste un **refuge** (colonie non tenue par un ennemi, sans armée ennemie, à
  `neutral_radius_steps` × 140 km de marche sans traverser de ZdC ni de place ennemie : la règle « neutre »
  de C7a, inutilisée depuis M2) et si l'armée n'a pas perdu `heavy_defeat_losses_percent` (42, nouveau,
  `data/settlements/rules.json`) de ses hommes dans la bataille. Sinon débandade (règle C7a inchangée).
  `retreat_target` garde sa signature (pertes 0).
- `road_crossing_radius_km` (6) dans `data/movement/rules.json` (pipeline seulement ; `None` côté Rust =
  tous gardés).
- Morceaux d'un même fleuve dont les bouts sont à moins de 20 px carte reliés (`join_river_gaps`).

## Écosse

Revenu ≈ entretien à la fin (495-873 livres de chaque côté selon la graine) et trésor négatif 6-8 graines
sur 8 dès avant M2 : le nombre de banqueroutes (8, 0, 15, 18, 4, 7 selon les variantes) suit le bruit des
levées et des pertes, pas une règle de mouvement. Hors périmètre (équilibrage économique).

## Limites

- Débandades à 1,8 / graine avec la nouvelle grille (1,2 avant M2) : plus de batailles, donc plus de défaites
  loin de chez soi. Seuil ajustable dans les données.
- Batailles en hausse (89 / graine contre 59 avant M2) : effet du tour séquentiel et des ponts-goulets.
- Limites de tracé M1 inchangées ailleurs (bras du delta du Rhin en cours d’eau secondaires).
- Les ponts ne peuvent être ni coupés ni gardés ; les gués ignorent la marée.

## Prochaine étape

Fusion de main dans la branche, puis vérifications finales (cargo, pytest, build.sh, Godot).
