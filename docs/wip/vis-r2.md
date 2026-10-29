# VIS-R2 : frontières-bande (état)

Spec : docs/audits/2026-10-29-visuel-campagne.md, lot R2. Worktree, non fusionné.

## État
- données `data/map/stance_fill.json` + schéma : bloc `band`, `zoom.flat_from_distance`, `flat_fade_distance`.
- `client/src/campaign/provinces/band.rs` : paramètres, poids, couleurs de bande, tests.
- `fill.wesl` : 8 taps (4 directions x rayons 1 et 2 texels) sur la texture de remplissage (quart de raster) ; texture de bande (binding 6) = couleur de position, alpha = alpha au bord.
- `overlay.rs` : peinture de la texture de bande, uniform `band` (largeur px, falloff, poids d'aplat, unité monde par texel).

## Fait (reprise)
Rebasé sur main. clippy -D warnings vert, `cargo test --features dev` vert (1510 lib dont shader_check, band.rs), captures camp_guyenne / camp_calais / camp_2600 prises (hors dépôt, scratchpad).
Mesures : critère 1 Guyenne vs Saintonge (recadrages 300x150) saturation 0,152 vs 0,179 (-15 %), valeur 0,327 vs 0,406 (-19 %) ; le cœur n'a aucun lavis (r-g = 0), l'écart est le sol sombre (R1/R3).
Critère 2 : transect x=900 sous la frontière, rouge r-g 70 -> 51 -> 24 -> 7 sur ~16 px (ruban compris). Critère 3 : à d 2600 le poids d'aplat vaut 1, la branche bande est sautée (ancien chemin exact), non comparé pixel à pixel. Critère 4 : banc GPU non mesuré.
Restent : banc pan d 300, pytest schémas, renommage XXXX de l'ADR à la fusion.
