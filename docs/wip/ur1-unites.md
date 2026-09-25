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

## État : terminé (non fusionné), `main` fusionné (03a42b82), tests, clippy, pytest, smoke verts.

## Points ouverts

- Couleuvriniers : la simulation tire encore des flèches (`sim.rs::missile_kind` décide par id) ;
  il faudrait un type de projectile « balle de plomb » (fumée, son) côté cœur et rendu.
- Jinetes : pas d'animation de lancer de javeline (repos pendant le tir).
- Routiers et écorcheurs : 3,4 k et 3,7 k triangles au LOD0 (au-dessus du budget de 2,4 k).
- Pas de capture en bataille simulée : la démo de `battle_scene.gd` ne choisit pas ses types.
- Bombardes du XVe siècle / artillerie de campagne non ajoutées (engins : lot SG1).
- Les types du XVe siècle ne sont atteints qu'au-delà de 400 tours ; `century_probe` non relancé.
- `tools/cent_ans_tools/budget.py` ne lit pas un fichier à deux tables (section session 7) :
  la dépense UR1 a été consignée à la main.
