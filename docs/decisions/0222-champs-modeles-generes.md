# 0222 : champs, vergers et vignes en modèles 3D générés (DN-CHAMPS)

Numéro provisoire (renumérotation à la fusion).

## Contexte
Le sol peint (`hb_ground.gdshaderinc`, ADR 0213) donne des parcelles de Voronoi et une culture dominante
par région, mais seulement en texture. Le joueur veut de vrais champs (blé, vigne, vergers) faits avec
les glb `crop_*`, `tree_*`, `econ_*` générés (ADR 0212).

## Décision
- Nouvelle couche `FieldLayer` (placement `FieldPlan`) : un MultiMesh par modèle, cellules de 2 px
  planifiées à la demande sous budget de ms, transformations mises en cache par cellule.
- Choix de culture partagé par les **données**, pas par le code : poids de cultures par biome
  (`ground_biome_mix.json`) et par paysage régional (`agri_landscapes.json` + masque) ; la table
  matière de sol -> modèles est `data/art/dn_fields.json` (schéma). Même règle de tirage que le shader
  (région de Voronoi large, 70 % de parcelles à la culture dominante, le reste au tirage propre).
- Le hachage du shader (`fract(sin(...))`) n'est pas reproductible côté CPU (précision flottante) : la
  couche utilise un hachage entier. Conséquence : la culture dominante d'une région suit la même table
  et le même mécanisme, mais pas parcelle par parcelle le tirage du shader. Sous les modèles, le sol
  peint reste un champ cultivé ; en champs, les modèles masquent le détail.
- Forêt dominante : une parcelle n'est cultivée que si la part de cultures du splat l'est et la part
  de forêt du splat est sous `forest_max` ; ni villes (`town_clear_px`), ni lits de fleuve.
- Tailles grossies (style maquette, comme les villes), portée `view_range_units` : rien au palier parchemin.

## Conséquences
Coût : voir `docs/wip/dn/champs.md`. À reprendre si le joueur voit un désaccord sol/modèles : cuire
une carte de culture dominante (PNG) que le shader et `FieldPlan` liraient tous deux.

## Révision : une parcelle = un modèle (retour joueur 10-09)
Motif : le joueur a rejeté le rendu (« ça ne va pas du tout, il faut faire un asset du champ entier et pas
d'un carré de blé ou une vigne »). Les touffes `crop_*` grossies et chevauchées donnaient un rendu en éclats.
- Décision : chaque culture a un modèle de **parcelle entière** `props/field_<clé>` (maquette plate, base de
  terre fine, rangs/grille/terrasses/casiers sur toute l'emprise), généré par le procédé classique
  (Z-Image Turbo fal -> TRELLIS 1 `fal-ai/trellis`, une image, une graine, ni vues dos/côté ni multivue).
  Catalogue `data/art/dn_catalog_fields.json`, classe d'ingest `field` (axe long, base conservée).
- `FieldPlan` pose **une instance par parcelle cultivée**, au site de la parcelle ; plus grande dimension =
  `(parcel_px - headland_px) * footprint_fill` ; lacet = repère des rangs de la région (+ `yaw_jitter`),
  taille +- `size_jitter`. Disparus : `spacing_m`, `row_m`, `height_scale`, `size_m`, accents (pressoirs,
  pergolas). Plafond d'instances 14 000 -> 1 500.
- Conséquence : sans glb `field_*` ingéré, la couche ne dessine rien (modèles absents ignorés).
