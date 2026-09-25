# WIP — UR1 : variété des unités (roster à la Medieval II)

Branche : `worktree-agent-a213f64dfca3b3a18`.

## État

| Lot | État |
|---|---|
| Données : 14 types, schéma (`available_from`, `available_until`, `figure`), doctrines, `enables_units` | fait |
| Cœur : refus de recrutement hors époque (`orders.rs::recruit_blocker`), tests `sim-campaign/tests/ur1_units.rs` | fait |
| Équilibre : sondes `matrix`, `campaign`, `rt` | fait (voir plus bas) |
| Figurines (pipeline V2) : 13 nouvelles (infantry_3-8, archer_3-5, cavalry_3-6), style et noblesse dans le manifeste, `BattleMeshes.variant_of` lit `figure` | fait |
| Coups critiques : armes des 14 types dans `data/fx/battle_gore.json` | fait |
| Cartes d'unité : 14 illustrations (0,64 $, `docs/budget.md` session 7), 12 icônes game-icons.net, `CREDITS.md` | fait |
| Encyclopédie et infobulles : époque, factions, mercenaires | fait |
| Captures `docs/audit/captures/ur1/` (rangs hors simulation : repos, mêlée, tir) | fait |

## Équilibre

Retouches hors nouveaux types : hommes d'armes à pied 950 / 90 → **1 050 / 100** (81-83 % sinon),
piquiers flamands 600 / 55 → **650 / 60** (85 % sinon), sergents montés 750 / 70 → **700 / 65**
(18 % sinon). La matrice est très sensible aux seuils (nombre entier d'unités pour 9 000 livres).

Matrice à budget égal (victoires moyennes, 23 types) : chevaliers 80, retenues anglaises 76,
arbalétriers gascons 77, schiltron 77, gendarmes d'ordonnance 69, goedendag 67, archers longs 60,
piquiers flamands 59, francs-archers 58, arbalétriers 57, hommes d'armes 56, chevaliers bretons 55,
hobelars 51, génois 47, lanciers gallois 40, écorcheurs 35, jinetes 31, couleuvriniers 30,
coutiliers 29, milice 28, routiers 27, archers montés 26, sergents montés 23 : **toutes entre 20 et 80 %**.

Campagne `200 1..8` (main 8c058417 → UR1) : milice 34,5 → **32,7 %** ; archers longs 51 → **60 %**
des recrutements anglais ; types recrutés par partie (min) 9 → **15** ; guerre FR-EN 33 → 36 % ;
batailles 217 → 206 ; changements de propriétaire 6,9 → 7,4. Nouveaux types recrutés par l'IA :
jinetes, routiers, schiltron, goedendag, hobelars, chevaliers bretons, retenues anglaises (les
types du XVe siècle n'apparaissent pas en 200 tours, soit 1337-1387).

`rt 3` (3D contre auto-résolution) : 19 / 20 comme main (désaccord piquiers contre hommes d'armes
déjà présent sur main).

## Nouveaux types

| Id | Nom | Accès | Figurine |
|---|---|---|---|
| unit_welsh_spearmen | Lanciers gallois | culture galloise | infantry_3 |
| unit_hobelars | Hobelars | cultures anglaise, galloise, irlandaise ; jusqu'en 1400 | cavalry_6 |
| unit_english_retinue | Hommes d'armes des retenues | Angleterre, tech. hommes d'armes à pied | infantry_7 |
| unit_scottish_spearmen | Schiltron écossais | culture écossaise | infantry_4 |
| unit_goedendag_militia | Milice au goedendag | cultures flamande, néerlandaise | infantry_5 |
| unit_coutiliers | Coutiliers | France, Bretagne, Bourgogne ; tech. compagnies d'ordonnance ; dès 1445 | infantry_6 |
| unit_francs_archers | Francs-archers | France ; tech. francs-archers ; dès 1448 | archer_3 |
| unit_ordonnance_gendarmes | Gendarmes des compagnies d'ordonnance | France ; tech. ; dès 1445 | cavalry_3 |
| unit_routiers | Routiers des Grandes Compagnies | mercenaires, 1356-1395 | infantry_8 |
| unit_ecorcheurs | Écorcheurs | mercenaires, cultures française, occitane, allemande ; 1435-1445 | cavalry_4 |
| unit_jinetes | Jinetes | cultures castillane, andalouse, galicienne | cavalry_5 |
| unit_breton_knights | Chevaliers bretons | culture bretonne | cavalry_0 (chevaliers) |
| unit_gascon_crossbowmen | Arbalétriers gascons | cultures occitane, basque, navarraise | archer_4 |
| unit_culveriners | Couleuvriniers | tech. couleuvrines, arsenal ; dès 1380 | archer_5 |

## État : UR1 terminé (non fusionné). UR2 (5 tâches ci-dessous) terminé, `main` fusionné,
fmt/clippy/tests (core, 395 pytest), build.sh, import, smoke verts.

## Points ouverts

- ~~Couleuvriniers : la simulation tire encore des flèches...~~ **fait (UR2, tâche 1)** : champ
  `missile` (`arrow|bolt|bullet|javelin|stone`) dans `unit_type.schema.json`, lu par
  `UnitType -> UnitSetup -> Unit`, `BattleSim::missile_kind`/`missile_cause` le préfèrent à
  l'ancienne heuristique id/pavois (repli conservé pour les types qui ne le renseignent pas).
  `unit_culveriners.json` → `bullet`, `unit_jinetes.json` → `javelin`. Rendu BV1
  (`battle_volleys.gd`, `battle_volley.gdshader[inc]`, `battle_stuck_arrow.gdshader`) : code de
  paquet élargi à 2 bits de sorte, vitesse/arc/étalement et ruban dédiés, fumée de mise à feu
  (panneau flamme réutilisé, gris, fixée à la bouche) pour la balle, traits fichés et décoration
  des corps mis à jour. Son : pas d'échantillon dédié dans `sound_bank.json`, repli sur
  `bombard`/`bow_release` à gain réduit (`battle_volleys.gd`, `battle_audio.gd`).
- ~~Jinetes : pas d'animation de lancer de javeline...~~ **fait (UR2, tâche 2)** : clips
  `c_javelin_idle/walk/throw` (`battle_skinned_poses.py`, `battle_skinned_cavalry.py`), style
  `horse_javelin` (`battle_skinned.gd`), `cavalry_5` reconstruit.
- ~~Routiers et écorcheurs : 3,4 k et 3,7 k triangles...~~ **fait (UR2, tâche 3)** : sous 3 000
  triangles au LOD0 (`infantry_8` 2 980, `cavalry_4` 2 952 ; correction d'un budget mounted mal
  appliqué pour `cavalry_4`, allègement de la coiffe pour `infantry_8`).
- Pas de capture en bataille simulée : la démo de `battle_scene.gd` ne choisit pas ses types.
- Bombardes du XVe siècle / artillerie de campagne non ajoutées (engins : lot SG1).
- ~~Les types du XVe siècle ne sont atteints qu'au-delà de 400 tours...~~ **fait (UR2, tâche 5)** :
  `century_probe 464 1 2 3 4` (release) relancé, voir résultats ci-dessous.
- ~~`tools/cent_ans_tools/budget.py` ne lit pas un fichier à deux tables...~~ **fait (UR2, tâche
  4)** : lit toutes les tables (sections `## Session N`), cumul et plafond par session courante,
  `session_totals()`, `add_entry` écrit dans la dernière table. Tests dans `tools/tests/
  test_budget.py` (nouvelles fixtures multi-sessions), suite complète verte (373 tests).

## UR2 (suite du lot, agent séparé)

1. Projectile selon les données (voir ci-dessus) : **fait**, commit `5d5edd2d`.
2. Jinetes, clip de lancer de javeline (Blender V2) : **fait**, commit `9668931a`.
3. Budget de triangles (routiers, écorcheurs) : **fait**, commit `cde0e583`.
4. `tools/.../budget.py` (tables multi-sessions) : **fait**, commit `4a205183`.
5. `century_probe` 4 graines × 464 tours, chiffres XVe siècle : **fait**, commit `b5da244a`
   (sonde) + résultats ci-dessous.

### `century_probe 464 1 2 3 4` (build release, après UR1+UR2)

Survie en 1400 : england 4/4, france 4/4, burgundy 4/4, scotland 4/4 (**4 majeures 4/4**, comme
avant UR1/UR2 ; pas de régression).

Types du XVe s. recrutés (au moins une fois, une des 4 graines à 464 tours = jusqu'en 1453) :

| Type | Recruté |
|---|---|
| Gendarmes d'ordonnance (`unit_ordonnance_gendarmes`, dès 1445) | 4/4 |
| Francs-archers (`unit_francs_archers`, dès 1448) | 3/4 |
| Coutiliers (`unit_coutiliers`, dès 1445) | 3/4 |
| Couleuvriniers (`unit_culveriners`, dès 1380) | **0/4** |

Les gendarmes d'ordonnance (le remplacement des hommes d'armes) sont systématiquement adoptés.
Francs-archers et coutiliers, plus tardifs (dès 1448/1445, soit 5-8 ans avant la fin de la sonde
en 1453), manquent une graine sur quatre — cohérent avec leur fenêtre courte plutôt qu'un signe
de déséquilibre. **Couleuvriniers jamais recrutés sur les 4 graines** malgré une disponibilité
dès 1380 (bien avant la fin de la sonde) : à investiguer (lot suivant) — hypothèses : coût/entretien
peu compétitif face aux archers/arbalétriers dans la matrice à budget égal (30 % de victoires
contre 57-77 % pour les archers, cf. § Équilibre plus haut), `required_technology: tech_handgonnes`
+ `required_building: bld_armoury` rarement construits par l'IA avant 1453, ou le recruteur IA ne
considère pas encore ce type. Pas un blocant pour ce lot (le rendu et les données sont corrects,
cf. tâche 1), mais à noter pour un futur lot d'équilibre (G-suivant).

Guerre FR-EN moy. 38 % [30-56] (1/4 dans la bande cible 55-75 %, déjà hors bande avant G1, cf.
audit § 6, sans lien avec UR1/UR2). Batailles FR/EN, banqueroutes, appels aux armes : dans les
ordres de grandeur habituels de ce probe. Détail complet dans le journal d'agent (log non conservé
au-delà de cette session ; relancer `cd core && cargo build --release -p ai --example
century_probe && ./target/release/examples/century_probe 464 1 2 3 4` pour le reproduire, environ
110 s pour les 4 graines en parallèle sur cette machine).
