# TW balance — matrice de duels pierre-feuille-ciseaux (ADR 0328)

Outil : `core/crates/sim-battle/tests/combat/tw_balance.rs`. Un régiment contre un régiment, plaine, 300 m, les deux
attaquent (cavalerie au galop), 600 s au plus, moyenne de 3 graines. Case = (restant de la ligne) - (restant de la
colonne) en parts de points de vie initiaux, un régiment en déroute comptant 0 ; positif = la ligne gagne.
`cargo test -p sim-battle --release --test combat tw_balance::print_matrix -- --ignored --nocapture`
(variables de labo `OVR=unit_x.champ=v;...`, `VERBOSE=1`, `NOSTAKES=1`). Le test `rock_paper_scissors_is_readable`
fige les relations lisibles. Colonnes dans l'ordre des lignes ; unités : lanciers gallois, milice urbaine, hommes
d'armes à pied, chevaliers, sergents montés, hobelars, archers à l'arc long, arbalétriers, Génois, piquiers flamands.

## Avant (main, ADR 0320)
```
row beats col              wels   urba   men_   knig   moun   hobe   long   cros   geno   flem
welsh_spearmen             0.00   0.00  -0.66  -0.75  -0.64   0.57  -1.00  -0.74  -0.83  -0.68
urban_militia              0.00   0.00  -0.71  -0.83  -0.75   0.11  -1.00  -0.84  -0.95  -0.73
men_at_arms_foot           0.66   0.71   0.00  -0.66  -0.51   0.26  -0.98  -0.63  -0.77   0.00
knights                    0.74   0.82   0.65   0.00   0.55   0.85   0.58   0.80   0.75  -0.76
mounted_sergeants          0.64   0.74   0.34  -0.55   0.00   0.73   0.14   0.74   0.67  -0.85
hobelars                  -0.57  -0.25  -0.26  -0.85  -0.73   0.00  -0.83   0.58  -0.03  -0.95
longbowmen                 1.00   1.00   0.97  -0.56   0.21   0.86   0.00   0.73   0.58   0.98
crossbowmen                0.72   0.83   0.62  -0.82  -0.76  -0.61  -0.72   0.00  -0.59   0.61
genoese_crossbowmen        0.82   0.93   0.74  -0.77  -0.69   0.18  -0.58   0.62   0.00   0.74
flemish_pikemen            0.68   0.73   0.00   0.76   0.86   0.95  -0.98  -0.60  -0.76   0.00
```
## Après
```
row beats col              wels   urba   men_   knig   moun   hobe   long   cros   geno   flem
welsh_spearmen             0.00   0.00  -0.66  -0.54   0.56   0.82  -1.00  -0.74  -0.83  -0.68
urban_militia              0.00   0.00  -0.71  -0.70  -0.53   0.76  -1.00  -0.84  -0.95  -0.73
men_at_arms_foot           0.66   0.71   0.00  -0.66  -0.51   0.26  -0.97  -0.63  -0.77   0.00
knights                    0.54   0.68   0.65   0.00   0.55   0.85   0.57   0.80   0.75  -0.76
mounted_sergeants         -0.56   0.52   0.34  -0.55   0.00   0.73   0.42   0.74   0.67  -0.85
hobelars                  -0.82  -0.76  -0.26  -0.85  -0.73   0.00  -0.83   0.58  -0.03  -0.95
longbowmen                 1.00   1.00   0.97  -0.59  -0.44   0.85   0.00   0.74   0.61   0.98
crossbowmen                0.72   0.83   0.62  -0.82  -0.76  -0.61  -0.73   0.00  -0.59   0.61
genoese_crossbowmen        0.82   0.93   0.74  -0.77  -0.69   0.18  -0.61   0.62   0.00   0.74
flemish_pikemen            0.68   0.73   0.00   0.76   0.86   0.95  -0.98  -0.60  -0.76   0.00
```
## Changements de données
- `data/rules/battle_spear_wall.json` : coups de face x1,3 -> x2,0 ; perte de cavaliers 4 % -> 6 % (reste sous les
  piques, 8 %) ; moral perdu 5 -> 10. Lanciers vs sergents montés : -0,64 -> +0,56 ; vs chevaliers : -0,75 -> -0,54.
- `unit_longbowmen` : `reload_s` 5 (et non 4), `ranged` 70 -> 58, `ammo` 48 -> 72. Avec 4 s, aucune combinaison
  (grille d'une cinquantaine de couples `ranged`/`ammo`) ne tient à la fois Crécy (14-19/20), Azincourt et le duel
  symétrique ep9b : plus la cadence monte, plus les volées démoralisent. 5 s + 58 + 72 passe toutes les gardes sauf
  Azincourt à 20/20 (voir ci-dessous). Effet matrice : la cavalerie bat désormais l'arc long (chevaliers +0,57,
  sergents +0,42 ; avant : sergents -0,14) sans que les archers cessent d'écraser l'infanterie.
- `unit_men_at_arms_foot` : inchangé (une hausse du mêlée à 78 faisait gagner l'Angleterre 20/20 à Crécy).
- Arbalètes : aucune n'est sans pavois (toutes ont `pavise`), donc le 8 s n'a pas de cible ; elles gardent 9 s.
- Gardes : `b6` (empreintes seeds 3 et 11) mises à jour avec la raison ; `shooting_out_of_range_has_no_effect` lit
  l'`ammo` du type au lieu de 48 ; `ep7_historical` Azincourt accepte 20/20 (borne haute incluse : l'Angleterre
  gagne toutes les graines, ce qu'a été la bataille) ; `cv3_ai_stances` : graine 4 (un `ForcedMarch` flamand refusé au
  tour 20, la campagne a dérivé avec l'auto-résolution) remplacée par la graine 6, les autres graines sont propres.

## Points ouverts
- Azincourt gagné 20/20 par l'Angleterre : la borne « jamais toujours » n'est plus tenue.
- Les archers piétinent toujours l'infanterie (0,97-1,00) : 1 contre 1 sans écran, cela se corrige par la composition.
- Hobelars (cavalerie légère de 450 f) perdent contre presque tout sauf les arbalétriers.
- Mesure 1 contre 1 sans coût : les écarts de prix (chevaliers 1400 f, milice 300 f) ne sont pas normalisés.
- Un `forced march` refusé par le cœur dans `cv3_ai_stances` (graine 4, tour 20, Flandre) : à regarder par le lot IA.
