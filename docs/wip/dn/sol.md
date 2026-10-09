# DN-SOL — parcellaire du sol

## Fait
Cause : `hb_ground.gdshaderinc` (grille tournée par blocs, petites cellules, culture aléatoire). Remplacé par
Voronoi irrégulier + régions dominantes + clairières (ADR 0213). Captures : /private/tmp/claude-501/dnsol{1..5}.
Exposé : `hb_clearing_splat(s, terroir, p, slope)` ; uniformes `hb_clear_floor`, `hb_region_cells`.
Accord DN-FORET : la forêt est à eux ; les champs perdus deviennent prés (R) et landes (A) dans le splat,
donc DN-FORET peut semer plus de forêt là où `terroir.r` est faible sans conflit visuel.

## Pour une session locale
Rien d'obligatoire. Option : textures de cultures dédiées (blé mûr, orge verte, vigne en rangs) via
`fal-ai/z-image/turbo` (non faites : non nécessaire ; pas de repli local demandé).

## Reste
Variété de couleurs encore modérée (palette de saison) ; `field_at` de près inchangé.
