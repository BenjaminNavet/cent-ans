# ADR 0031 — Échelle massive des batailles

Date : 2026-09-25. Statut : accepté. Lot EP1 du chantier « batailles épiques »
(`docs/wip/epic.md`, suivi `docs/wip/ep1-echelle-massive.md`).

## Contexte

Le joueur veut des batailles de 15 000 soldats et plus. Le champ était figé à 1200 × 800 m
(constantes `FIELD_WIDTH` / `FIELD_DEPTH`, lignes de déploiement à z = 250 et 550) et lu en dur
par le relief, le site (B5), l'IA, les renforts, le terrain rendu, la caméra, l'herbe couchée…
Un camp ne pouvait aligner que 20 régiments à la fois (`MAX_ON_FIELD`, F5d) : ≈ 4 800 soldats au
plus sur le terrain. Les lots EP2, EP3, EP6 et EP7 doivent pouvoir s'appuyer sur un champ de taille
variable.

## Options

- **A. Agrandir le champ unique** (par exemple 2400 × 1600 pour tous) : simple, mais les
  escarmouches se perdent dans un champ vide, toutes les graines changent (relief, site, IA) et les
  tests de régression seraient à refaire.
- **B. Paliers d'échelle selon l'effectif, dans les données** : le premier palier garde exactement
  l'ancien champ ; les batailles plus grosses ont un champ plus grand et plus de régiments présents.
- **C. Champ continu proportionnel à l'effectif** : pas de palier, mais chaque bataille a une taille
  différente, ce qui complique l'équilibrage (distances d'IA, zones) et les tests.

## Décision

**Option B.** `data/rules/battle_scale.json` (schéma `battle_scale_rules.schema.json`, embarqué
dans `sim-battle`, module `scale`) définit des paliers par effectif total (soldats simulés des deux
camps, réserves comprises) :

| Palier | Effectif | Champ | Écart des lignes | Zone de déploiement | Régiments / camp |
|---|---|---|---|---|---|
| `skirmish` (escarmouche) | ≤ 4 000 | 1200 × 800 m | 300 m | 300 m | 20 |
| `large` (grande bataille) | ≤ 8 000 | 1800 × 1200 m | 340 m | 400 m | 40 |
| `epic` (bataille rangée) | au-delà | 2400 × 1600 m | 380 m | 500 m | 80 |

- `FieldSize` porte largeur, profondeur, écart des lignes et profondeur des zones ; les lignes sont
  centrées (`profondeur/2 ∓ écart/2`), la zone de déploiement a son bord avant 50 m au-delà de la
  ligne. Toutes les grandeurs dérivées (centre, demi-largeur dégagée de 360 m, bandes des vallons,
  couloirs des renforts…) valent **exactement** les anciennes constantes au premier palier : mêmes
  graines, mêmes batailles, tests inchangés.
- Sur un champ plus grand, le nombre de collines, bois, boues, escarpements et fossés de marais
  croît avec la surface, celui des vallons avec la largeur, celui des couloirs d'entrée des
  renforts avec la largeur.
- Les sièges gardent toujours le premier palier (plan de ville posé sur le champ standard).
- Le surplus au-delà du plafond de régiments attend toujours en renfort (F5d).
- `BattleSim::new_scaled` / pont `set_scale_tier(clé)` forcent un palier (cartes historiques EP7,
  bancs `--scale=`).
- Rendu : `get_terrain()` exporte `width`, `depth` et les lignes ; `BattleTerrain` (splatmap,
  anneaux), caméra (bornes, recul maximal), minicarte, brume au sol, herbe couchée et boîtes
  englobantes des effets suivent la taille du champ.
- Taille des unités : option **Épique (× 4)** en plus de Petite à Ultra (ADR 0016, rendu seul).

## Conséquences

- Les constantes `FIELD_WIDTH`, `FIELD_DEPTH`, `ATTACKER_LINE_Z`, `DEFENDER_LINE_Z`, `MAX_ON_FIELD`
  et `ZONE_DEPTH` ne décrivent plus que le champ standard (tests, sièges) : le code lit
  `Battlefield::size` et `BattleSim::scale()`.
- Les batailles de campagne au-delà de 4 000 soldats changent de champ (donc de relief) ; leurs
  résultats bougent par rapport aux versions précédentes.
- Le coût de simulation croît avec le nombre de régiments présents (sonde IA contre IA à 60 contre
  60 : voir le suivi) ; le coût de rendu est traité par les paliers de distance des figurines (voir
  « Rendu » dans le suivi).
