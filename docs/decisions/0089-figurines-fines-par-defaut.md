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
   toujours le LOD1. Un second calque (`BattleSoldiers._fine_near`) dessine le LOD0 des seuls
   soldats à moins de `FINE_DETAIL_DISTANCE` = 12 m × préréglage (`battle_lod`) **et** dans le
   champ de la caméra (sphère de 2,5 m contre les plans du frustum) : tampon compacté côté CPU,
   rang d'origine du soldat en donnée perso (`INSTANCE_CUSTOM.x`, `instance uniform bool
   id_in_custom`) pour garder visage, variante et clip. Le shader reçoit une bande de distance par
   calque (`instance uniform vec2 lod_band`, caméra → soldat) : hors de sa bande, un soldat est
   replié en un point avant le skinning ; c'est elle qui tranche exactement (le tri CPU prend 1 m
   de marge), si bien qu'aucun soldat n'est dessiné deux fois ni oublié. Aucun des deux calques
   ne porte d'ombre (le LOD2 la porte, comme avant). Les figurines Quaternius gardent le LOD par
   régiment.

   Essai écarté : un calque LOD0 avec le tampon entier et le repli dans le shader seulement.
   Le repli précoce ne suffit pas : 709 soldats × 10 k sommets par image coûtaient encore
   +8 ms dans le banc rapproché.
2. **LOD1 et LOD2 allégés** (`battle_fine_figures.TRI_CAP` / `RIDER_CAP`,
   `battle_fine_cavalry.HORSE_LOD`), recuisson complète (`battle_fine.py -- bake`).
3. **Plafond de triangles relevé** (bible DA § 6) pour les figurines fines, chiffres réels du
   manifeste :

   | Famille | LOD0 | LOD1 | LOD2 |
   |---|---|---|---|
   | à pied (20 recettes) | 9 380-11 872 | 1 273-1 319 (avant : 1 784-1 968) | 235-252 (avant : 320-345) |
   | monté (8 recettes, cheval compris) | 15 193-17 414 | 1 885-2 055 (avant : 2 749-2 933) | 446-534 (avant : 599-650) |

   Plafonds de la bible § 6 : ≤ 12 000 / 17 500 au LOD0, ≤ 1 350 / 2 100 au LOD1, ≤ 260 / 550 au
   LOD2 (à pied / monté). Recettes : `TRI_CAP` (11 900, 1 350, 260), `RIDER_CAP` (9 000, 1 000,
   180), cheval LOD1 600 + crinière 60 + queue 36, LOD2 170 + 12 + 8.

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

Banc `--units=50 --benchmark --bench-at=90`, 1600×900, M4 Pro, 4 passes alternées par
configuration, durée moyenne d'image (écart-type entre passes ≤ 0,06 ms en Ultra) :

| Préréglage, vue | Quaternius (`--coarse-figures`) | fines avant FG5 | fines FG5 (défaut) |
|---|---|---|---|
| Ultra, standard | 16,58 ms | 18,39 ms (+8,3 %) | **17,04 ms (+2,8 %)** |
| Ultra, standard, `--no-fg3` | — | 17,86 ms (+5,2 %) | 16,76 ms (+1,1 %) |
| Ultra, rapprochée (`--closeup`) | 15,15 ms | 29,88 ms (+94 %) | **18,09 ms (+19 %)** |
| Élevée, standard | 10,12 ms | — | 10,68 ms (+5,4 % ; +1,3 % sans une passe à 11,9) |
| Élevée, rapprochée | 10,26 ms | — | 11,91 ms (+16 %) |

(Les colonnes « avant FG5 » viennent de séries du même jour, 4 passes chacune.) En Élevée,
l'affichage plafonne vers 10 ms : l'écart y est moins lisible.

- Objectif tenu dans la vue standard (≤ +5 % en Ultra). En vue rapprochée, les figurines
  fines restent plus chères (+16 à +19 %). C'est le prix du LOD0 et du LOD1 fins, qui sont
  réellement vus : 82 soldats en LOD0 dans le champ, 8 régiments en LOD1. Sans LOD0 du tout,
  l'écart serait de +7,6 %.
- `fine_distance` (80 m) et tuiles de détail : aucun effet mesurable (aucune figurine fine en
  LOD0 ou LOD1 dans la vue standard ; en vue rapprochée, couper les tuiles ne change rien). Ils
  restent tels quels.
- Coût CPU : le tri des soldats proches (GDScript) ne parcourt que les régiments dont le centre
  moins la demi-diagonale (+ 4 m) est dans le rayon : quelques centaines de soldats par image au
  plus, en vue rapprochée.
- Le LOD0 bascule soldat par soldat autour de 12 m (× préréglage). Les soldats hors champ n'ont
  pas de LOD0 : un soldat qui entre dans le champ par le bord passe au LOD0 à l'image suivante.
- `--coarse-figures` et le pipeline Quaternius (`battle_skinned/`) sont à retirer après la
  transition ; `--legacy-figures` (figurines rigides) est inchangé.
- Banc rapproché disponible pour tout lot futur : `--benchmark --closeup`. Le JSON du banc
  donne `lod_counts` (régiments par niveau, calques LOD0 fins et soldats qu'ils dessinent).
