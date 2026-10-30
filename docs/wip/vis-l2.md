# VIS-L2 habitat à l'échelle (ADR 0824)

Spec : docs/audits/2026-10-30-juge-visuel-iter3.md, G1-2 et lot L2.

## État (30/10)
Fait et testé (cargo check + tests `campaign::` verts, pytest schéma vert) :
- `max_real_mult` (10 props d'habitation, 3 × réel, portée 70-120) ; `hamlet_core` (décalque drapé, 4 palettes) ;
  `hameau_jardins`/`hamlet_garden` supprimés ; `towns/shapes.rs` supprimé (maisons en kit glb à tout LOD, plus d'arbres
  de jardin bâtis) ; test d'inventaire ; `folk_scale` 675 → 300.
Non fait : maquette `village_west` sur le noyau, décalques Z-Image, mesure visuelle (captures), mesure de visibilité des
villes `city` (Bordeaux, Caen), Paris vs voisins.

## Prochaine étape
À la fusion : une capture camp_0200 / camp_calais / camp_guyenne (tools/compare_views.sh) pour juger le décalque (alpha,
rayon 220 m, palette) ; vérifier le coût d'entités des villes en kit à tout LOD ; fmt/clippy complets.
