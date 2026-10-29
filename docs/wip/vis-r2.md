# VIS-R2 : frontières-bande (état)

Spec : docs/audits/2026-10-29-visuel-campagne.md, lot R2. Worktree, non fusionné.

## État
- données `data/map/stance_fill.json` + schéma : bloc `band`, `zoom.flat_from_distance`, `flat_fade_distance`.
- `client/src/campaign/provinces/band.rs` : paramètres, poids, couleurs de bande, tests.
- `fill.wesl` : 8 taps (4 directions x rayons 1 et 2 texels) sur la texture de remplissage (quart de raster) ; texture de bande (binding 6) = couleur de position, alpha = alpha au bord.
- `overlay.rs` : peinture de la texture de bande, uniform `band` (largeur px, falloff, poids d'aplat, unité monde par texel).

## Pause demandée par le joueur
`cargo check --features dev` vert. Tests (`cargo test --features dev --lib provinces`, pytest schémas) lancés
puis arrêtés avant résultat (file d'attente heavy.sh saturée). Aucun résultat de test, clippy, ni capture encore.
Liens des données ignorées créés dans le worktree (data/map/pyramid, game/assets/models/dn).

## Prochaine étape
1. Relancer les tests ci-dessus, puis clippy `-D warnings` et `cargo test --features dev` (shader_check inclus).
2. Ajouter le champ band au dump `dump_provinces` si utile aux mesures.
3. Captures (3 max, 640 px, CENT_ANS_WEATHER=clear) camp_guyenne / camp_calais / camp_2600 via tools/compare_views.sh ;
   critère 1 en chiffres (Guyenne vs Saintonge), critère 3 (d 2600 identique : poids d'aplat 1 = ancien chemin),
   critère 4 banc pan d 300.
4. ADR : renommer XXXX à la fusion.
