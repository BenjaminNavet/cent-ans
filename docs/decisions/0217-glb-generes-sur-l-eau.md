# 0217 — glb générés sur et au bord de l'eau (lot DN-FLEUVE)

Date : 2026-10-09 (`docs/wip/dn/fleuve.md`). Numéro provisoire : à renuméroter à la fusion.

## Contexte

Navires (armées embarquées, lignes maritimes, fleuves), ponts, moulins, ports et oiseaux d'eau de la
carte de campagne étaient des maillages procéduraux ou des silhouettes. Le paquet de modèles générés
(ADR 0212) fournit des glb pour chacun.

## Décision

- Un registre et des règles en données : `data/art/dn_water_models.json` (schéma
  `art_dn_water_models.schema.json`). Les règles désignent des identifiants du registre ; le
  type de navire dépend du bassin (`SeaBasins`), de la culture de la faction, du nom du fleuve,
  de la structure du pont et de l'année (`from_year` / `until_year`).
- `DnWaterModels` mesure chaque glb (axe long rangé sur +X, longueur du registre en unités carte,
  pied à y = 0) : l'orientation et l'échelle ne sont pas codées par modèle.
- Repli systématique : table absente, identifiant inconnu ou glb non importé laissent l'ancien rendu.
- Les décors fixes (moulins, ports, chantiers, épaves) sont un `MultiMesh` par modèle et par tuile
  de 320 unités (`WaterPropsLayer`), visibles aux paliers près et moyen. Les ponts glb sont des
  enfants des ouvrages de `RiverCrossings` (portée du fleuve, axe du franchissement).
- Les oiseaux d'eau passent par `dn_model` dans `map_birds.json` : maillage cuit à plat, albédo
  baké échantillonné par `life_birds.gdshader`.
- Pas de second port-ville : les maquettes de lieux (camp-bati, DN-TROUS) gardent les bâtiments ;
  ce lot ne pose que jetées, grues, navires amarrés, chantiers et barques.

## Conséquences

- Sans le paquet de modèles, tout le lot est inerte (tests sautés avec un message).
- Sens de la proue non réglé par modèle (coques presque symétriques) ; `yaw_deg` existe pour corriger.
- L'année courante n'est pas connue de la carte : `DnWaterModels.year_override` à brancher.
