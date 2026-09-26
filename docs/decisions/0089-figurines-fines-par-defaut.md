# 0089 — Figurines fines par défaut, LOD0 par soldat (lot FG5)

Date : 2026-09-26. Statut : accepté. Suite des ADR 0014 (figurines skinnées) et 0088 (matières
cuites des figurines fines).

## Contexte

Les figurines fines (lots FG1 à FG4 : corps MakeHuman, équipement refait, cheval CC0, cartes
cuites FG3) étaient derrière `--fine-figures`. Elles dépassent de loin le plafond de la bible DA
§ 6 (≤ 3 000 triangles au LOD0 à pied, ≤ 3 500 monté) : 9 à 12 k à pied, 15 à 17 k monté.

Mesures avant FG5 (`--units=50 --benchmark --bench-at=90`, 1600×900, Ultra, M4 Pro) :

- banc standard : la caméra ne voit aucun régiment en LOD0 ou LOD1 (25 régiments en LOD2,
  75 en imposteurs). Écart fin/défaut **+8 %** en durée moyenne d'image, dû au LOD2 fin
  (335 triangles à pied, 600 monté, contre 230 et 340) et à la variante `FG3_BAKED` ;
- banc rapproché (nouveau, `--closeup` : caméra à 26 m d'un régiment) : **+94 %** (29,9 ms
  contre 15,4 ; 10,6 M primitives contre 5,1). Sans LOD0 : +15 %. Le LOD se choisissait par
  régiment, d'après son centre : un régiment en ligne (40 à 150 m de front) passait en entier
  en LOD0 dès que son centre était à moins de 24 m (× préréglage), soit 120 × 12-17 k triangles.

La médiane des durées d'image colle aux paliers de la cadence d'affichage (10 ms, 16,7 ms) ;
la métrique retenue est la durée moyenne (1000 / images par seconde), en passes alternées.

## Décision

1. **LOD0 par soldat pour les figurines fines.** Le calque principal d'un régiment fin dessine
   toujours le LOD1 ; un second calque (`BattleSoldiers._fine_near`) partage son tampon et dessine
   le LOD0. Le shader skinné reçoit une bande de distance par calque (`instance uniform vec2
   lod_band`, caméra → soldat) : hors de sa bande, un soldat est replié en un point **avant** le
   skinning. Rayon : `FINE_DETAIL_DISTANCE` = 12 m × préréglage (`battle_lod`). Le calque LOD0
   n'est montré que si un soldat au moins est dans le rayon (test sur le tampon, côté CPU). Les
   figurines Quaternius gardent le LOD par régiment.
2. **LOD1 et LOD2 allégés** (`battle_fine_figures.TRI_CAP` / `RIDER_CAP`,
   `battle_fine_cavalry.HORSE_LOD`), recuisson complète (`battle_fine.py -- bake`).
3. **Plafond de triangles relevé** (bible DA § 6) pour les figurines fines, chiffres réels du
   manifeste :

   | Famille | LOD0 | LOD1 | LOD2 |
   |---|---|---|---|
   | à pied (20 recettes) | TODO | TODO | TODO |
   | monté (8 recettes, cheval compris) | TODO | TODO | TODO |

   Le LOD0 n'est dessiné qu'à moins de 12 m (× préréglage) par soldat ; au-delà, le budget qui
   compte est celui du LOD1 et du LOD2 (ombres portées des régiments proches comprises).
4. **Bascule.** Le rendu fin devient le rendu par défaut (bataille, carte de campagne, décor du
   menu, cadavres, imposteurs, étendards, armoiries, blessés). `--coarse-figures` après `--`
   rend les figurines Quaternius le temps de la transition (`--legacy-figures` garde son sens :
   figurines rigides des lots B1/B4). `--no-fg3` : figurines fines sans cartes cuites.
   `--fine-figures` reste accepté, sans effet.
5. **Banc rapproché** : `--benchmark --closeup` place la caméra à 26 m du régiment du joueur le
   plus proche de l'ennemi.

## Conséquences

TODO (mesures après FG5).
