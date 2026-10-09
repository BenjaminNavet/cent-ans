# DN-CHAMPS : champs, vergers, vignes en modèles générés

État : fait (ADR 0220, numéro provisoire). Branche du worktree, non fusionnée.

## Fait
- `data/art/dn_fields.json` + schéma `art_dn_fields.schema.json` + `tools/tests/test_art_dn_fields_schema.py`.
- `FieldPlan` (placement pur, déterministe) et `FieldLayer` (MultiMesh par modèle, cellules de 2 px
  planifiées sous budget de 4 ms par image, transformations en cache), branchée dans
  `SettlementLayer.fields`. `--no-fields` coupe la couche. Rien au-delà de `view_range_units` (38).
- Culture : poids des tables du sol (biome + paysage régional), région dominante Voronoi 16 px,
  70 % de parcelles à la culture de la région. Cultures (blé, orge, seigle/avoine, lin/pastel, riz,
  fèves/choux), vignes (+ pressoirs/pergolas en accents), vergers et oliveraies en grille, terrasses.
- Forêt : parcelle non cultivée si splat forêt > `forest_max` ou part de cultures faible ; aucun
  champ près des villes ni sur un lit de fleuve.
- Remplissage : modèles grossis (640 m pour 1,6 px = 1150 m de parcelle), chevauchement, bord de
  parcelle visible, hauteur réduite (`height_scale`) pour un tapis, rangées dans le repère de la région.
- Tests : `game/tests/dn_fields_test.gd`, `tools/tests/test_art_dn_fields_schema.py`, `smoke.gd`.
- Mesure/capture : `game/tests/dn_fields_shot.gd` (via `tools/godot_bg.sh`).

## Coût (machine chargée, fenêtre 1280x720, ON contre OFF)
Beauce rig 12 : 24,0 contre 24,1 ms/image ; rig 5 : 26,3 contre 25,7 ms ; +10 appels de dessin
(un par modèle, 10 à 12 modèles), 6 000 à 7 000 instances (plafond 14 000), reconstruction <= 17 ms,
planification <= 3,3 ms par cellule (budget 4 ms par image). Version dense précédente (16 000
petites instances) : +3 à +5 ms. Pas de LOD de distance au-delà des 3 niveaux glb.

## Points ouverts
- Le shader garde son tirage `sin` : sol peint et modèles ne tirent pas la même culture parcelle par
  parcelle (voir ADR). Piste : cuire une carte de culture dominante lue par les deux.
- Pas de lanières (allongement par biome) ni d'orientation par parcelle hors région.
- Le modèle de blé est une touffe pointue : rendu en « éclats » de près ; un impostor/carte découpée
  dans l'asset donnerait un tapis plus lisse.
- Premier affichage : ~1 s de chargement des glb (préchargés en arrière-plan au setup).
