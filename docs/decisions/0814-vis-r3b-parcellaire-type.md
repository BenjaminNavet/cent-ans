# 0814 : parcellaire typé du sol de campagne (VIS-R3b)

Statut : proposé (lot VIS-R3b, boucle juge 2). Numéro à attribuer à la fusion.
Remplace 0213 et 0787 pour la forme des parcelles ; amende 0290, 0351, 0352 (cultures par saison, relief et eau gardés).

## Contexte

Le constat (b) de l'audit visuel du 29/10 : le sol de la carte de campagne est à 45 % de la carte de couleur (360 m par texel) et le
parcellaire est un diagramme de Voronoi isotrope. Il se lit comme des facettes gris-bleu, sans paysage agricole propre à chaque
pays. Contraintes dures : aucun échantillonneur de plus (limite Metal de 16, `docs/wip/bevy-camp-samplers.md`), tout le calcul
reste dans le cache de sol par nœud (ADR 0742), réglages en données avec schéma.

## Décision

1. **Part de la carte de couleur au plus près** : donnée `view.colormap_near_share` de `data/art/ground_biome_mix.json` (0,12 au lieu
   de la constante 0,45). À distance elle reste à 1.
2. **Cellules étirées et orientées** : la grille de Voronoi est remplacée par une grille de rangées déformée (`pc_cell`) : rangées de
   hauteur 1 décalées (`brick`), découpées en cellules de longueur propre (`len_var`), dans le repère tourné de la région (cap par
   sole, ADR 0213) et étiré par `aspect` (1:4 à 1:9 pour l'openfield). La déformation (`warp`) courbe les bords. La même grille
   sert aux soles du patchwork lointain.
3. **Formes par type de paysage** : bloc `landscapes` de `ground_biome_mix.json` (`default`, `openfield`, `bocage`, `midi`,
   `montagne`) ; chaque ligne de la table (biome ou paysage de `agri_landscapes.json`) porte `form`.
   - `openfield` : trois soles en couronnes autour des finages. Le masque de terroir (canal R, déjà chargé, déjà lié) donne la
     distance au noyau villageois sans nouvel échantillonneur : le tirage de culture de chaque cellule est tiré vers le centre de la
     couronne (`ring`). Stries de labour anisotropes (`furrow`) résolues seulement quand leur pas couvre quelques pixels.
   - `bocage` : cellules irrégulières, haie sombre de largeur écran `hedge_px` jusqu'à `hedge_range`.
   - `midi` : petites parcelles, vigne et olivier sur adrets (règles de relief ADR 0352 inchangées).
   - `montagne` : prés de fauche et alpage dominants.
4. **Saisons par classe** : la classe de culture d'une parcelle (blé, culture verte, labour, jachère, pré, foin) est tirée par saison
   selon `shares` de sa forme (`reassign` : part des parcelles concernées), puis colorée par `season_field_w` de cette saison. Plus
   de filtre de saison unique.
5. **Miroir CPU** : `terrain/parcel.rs` porte le hash entier, la grille, les couronnes et le tirage de classe, avec les tests purs
   (orientation, parts à ± 5 % sur 10 000 tirages).
6. **A/B** : `CENT_ANS_PERF_OFF=parcel` restaure le sol précédent (même binaire).

## Conséquences

- Le tampon uniforme `FieldParams` grandit de 31 vec4 (aucun échantillonneur, aucune texture).
- Le cache de sol par nœud est invalidé par le hash du matériau (les nouveaux champs en font partie).
- Les autres systèmes de parcelles (`fp_parcels`, vues de vallée) ne sont pas touchés.
- Le dump `campaign_terrain.material.fields` expose les formes et le paysage au foyer de la caméra.
