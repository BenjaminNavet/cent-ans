# BM terrain + camera (lot Bevy migration, branche bm/terrain)

Etat : camera, pyramide, quadtree (CDLOD via `relief-lod`), maillages CPU, colormap BC1 et
shader WESL portes ; verification par `--dump-state` (sections `campaign_camera`, `campaign_terrain`).

Prochaine etape : verifier le rendu reel (run_bg), puis streaming fin / reglages, puis
ce qui reste de `terrain.gdshader` (splat, saisons, brouillard, frontieres).
