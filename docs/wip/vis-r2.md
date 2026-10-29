# VIS-R2 : frontières-bande (état)

Spec : docs/audits/2026-10-29-visuel-campagne.md, lot R2. Worktree, non fusionné.

## État
- données `data/map/stance_fill.json` + schéma : bloc `band`, `zoom.flat_from_distance`, `flat_fade_distance`.
- `client/src/campaign/provinces/band.rs` : paramètres, poids, couleurs de bande, tests.
- `fill.wesl` : 8 taps (4 directions x rayons 1 et 2 texels) sur la texture de remplissage (quart de raster) ; texture de bande (binding 6) = couleur de position, alpha = alpha au bord.
- `overlay.rs` : peinture de la texture de bande, uniform `band` (largeur px, falloff, poids d'aplat, unité monde par texel).

## Prochaine étape
Tests, clippy, shader_check, mesures (critères 1-4), ADR, captures.
