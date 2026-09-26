# PB3b — Mise à l'échelle 3D MetalFX

Lot de PB3 (`pb3-performance.md`). Branche `worktree-agent-ab0e3b517d25d7cf4`. ADR 0080.
Rendu seulement (rien dans `core/`).

## État : terminé, en attente de fusion
- `RenderQuality` : clés `upscale_mode` / `upscale_scale`, `apply_upscale` (MetalFX sous Metal,
  FSR 1/FSR 2 ailleurs ; temporel ⇒ MSAA et FXAA coupés), `upscale_override` (bancs),
  `--upscale=` (captures). Préréglages : Basse spatial 0,67, Moyenne/Haute spatial 0,75,
  Ultra natif.
- Réglage joueur `video/upscale` (Automatique / Désactivée / MetalFX qualité 0,75 / MetalFX
  performance 0,5), Réglages > Affichage.
- Bancs : `release_journey --ab-configs=metalfx_s:…,metalfx_t:…,bilinear:…,off`, bataille
  `--bench-ab=off,metalfx_s:0.75,…`. Test `tests/pb3b_upscale_test.gd`.
- Mesures et verdict visuel : ADR 0080 ; captures `docs/img/pb3b/`.

## Points ouverts
- Temporel écarté : pluie effacée, fantômes des ailes de moulin (sommets animés par `TIME` sans
  vecteurs de mouvement). À reprendre si Godot expose la position précédente des sommets.
- Mesures sur machine chargée (séries plafonnées par le compositeur écartées) ; à refaire en
  plein écran Retina, où le gain devrait être plus grand.
