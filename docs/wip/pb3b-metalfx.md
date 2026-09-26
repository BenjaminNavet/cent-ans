# PB3b — Mise à l'échelle 3D MetalFX

Lot de PB3 (`pb3-performance.md`). Branche `worktree-agent-ab0e3b517d25d7cf4`. ADR réservé 0080.
Rendu seulement (rien dans `core/`).

## État
- [x] `RenderQuality` : clés `upscale_mode` / `upscale_scale` (tous préréglages : "off" pour
  l'instant), `apply_upscale` sur le viewport racine (MetalFX sous Metal, FSR 1/FSR 2 ailleurs ;
  temporel ⇒ MSAA et FXAA coupés), `upscale_override` pour les bancs.
- [x] Réglage joueur `video/upscale` (auto / off / quality / performance) + menu Réglages > Affichage.
- [x] Bancs : `release_journey --ab-configs=metalfx_s:0.75,metalfx_t:0.67,bilinear:0.75,off` ;
  bataille `--bench-ab=off,metalfx_s:0.75,…`.
- [ ] Mesures carte (d=1500, 491, 150, 40, Paris) et bataille, `--uncapped`, médianes de 3.
- [ ] Captures `docs/img/pb3b/` (carte proche : végétation, relief quadtree, fleuves ; bataille).
- [ ] Choix des valeurs par défaut (préréglages, `UPSCALE_PLAYER`), ADR 0080, tests.

## Prochaine étape
Mesures A/B.
