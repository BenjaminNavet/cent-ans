# ADR 0824 — VIS-L2 : habitat à l'échelle

Date : 2026-10-30. Statut : accepté (mesure visuelle à la fusion). Spec : `docs/audits/2026-10-30-juge-visuel-iter3.md`
(G1-2, lot L2). S'appuie sur les ADR 0222, 0803.

## Contexte
Les fermes de hameau de la couche campagne (`farm_*`, `house_brick`) étaient grossies à « taille écran » (`size_k` ×
d^0,85, ×1,7, plafond `max_mult` 60, ×2 de `countryside_size`, portée ×4 de `countryside_range`) : à d 200 elles
mesuraient 1 à 2 km et se lisaient comme des pions gris, plus hautes que Paris. Le potager (`hamlet_garden`) et les arbres
de jardin des villes (`towns/shapes.rs`) étaient des maillages bâtis par le code, donc low poly de fait.

## Décision
1. **Plafond d'échelle des habitations** (données) : champ `max_real_mult` d'une prop de `map_countryside.json`
   (schéma ajouté). Les huit props d'habitation sont plafonnées à 3 × leur taille réelle, ignorent les facteurs de
   maquette (`countryside_size`, `countryside_range`) et s'effacent à leur propre portée (fondu 70, disparition 120 de
   distance du rig). Plus aucune maison isolée au-delà de d 120.
2. **Noyau de hameau** (`countryside/hamlet_core.rs`, données `hamlet_core` de `map_countryside.json`) : un décalque au
   sol drapé sur le relief affiché (grille 12 × 12 sans bâti), rayon 220 m, retiré au-delà de d 260. Texture générée au
   chargement depuis quatre palettes (openfield, bocage, Midi, Nord) : terre battue, jardins en rangs, verger, cinq toits
   rouges vus d'en haut. Le potager `hamlet_garden` et sa règle sont supprimés.
3. **Villes** : `towns/shapes.rs` est supprimé. Les maisons prennent un modèle du kit glb à tout niveau de détail (le bloc
   n'est plus qu'un repli en boîte quand aucun modèle ne convient) ; les arbres de jardin ne sont plus bâtis, les arbres
   viennent de la végétation en place.
4. **Test d'inventaire** (`hamlet_core::tests::no_code_built_habitat_geometry`) : `shapes.rs` absent, aucun symbole de
   maillage de maison ou d'arbre de jardin dans `towns/` et `countryside/`, plus de `hamlet_garden` dans les données.

## Conséquences
- Moins d'instances au loin (fermes retirées au-delà de 120), gain de rendu neutre à positif ; les villes montrent leurs
  maisons en glb même en vue éloignée du calque 1:1 (coût d'entités à mesurer à la fusion).
- Non fait dans ce lot : maquette Meshy `village_west` posée sur le noyau, décalques Z-Image (le décalque est généré par
  code), plafond des figures et chariots relativement aux maquettes de ville, mesure de visibilité des villes `city`.
