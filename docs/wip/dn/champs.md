# DN-CHAMPS : champs, vergers, vignes en modèles générés

État : couche fonctionnelle (ADR 0220 provisoire), reste mesures de rendu et captures.

## Fait
- `data/art/dn_fields.json` + schéma `art_dn_fields.schema.json` + `tools/tests/test_art_dn_fields_schema.py`.
- `FieldPlan` (placement pur, déterministe) et `FieldLayer` (MultiMesh par modèle, cellules de 2 px
  planifiées sous budget de 4 ms par image), branchée dans `SettlementLayer.fields`. `--no-fields` coupe.
- Test : `game/tests/dn_fields_test.gd` (dominantes par région, forêt exclue, déterminisme, coût).

## Prochaine étape
Mesures de coût (voir ci-dessous), captures, rapport.

## Points ouverts
- Le shader garde son tirage `sin` : sol peint et modèles ne tirent pas la même culture parcelle par
  parcelle (voir ADR). Piste : cuire une carte de culture dominante lue par les deux.
