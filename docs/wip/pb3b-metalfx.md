# PB3b — Mise à l'échelle 3D MetalFX

Lot de PB3 (`pb3-performance.md`). Branche `worktree-agent-ab0e3b517d25d7cf4`. ADR réservé 0080.
Rendu seulement (rien dans `core/`).

## État
- [x] `RenderQuality` : clés `upscale_mode` / `upscale_scale` (tous préréglages : "off" pour
  l'instant), `apply_upscale` sur le viewport racine (MetalFX sous Metal, FSR 1/FSR 2 ailleurs ;
  temporel ⇒ MSAA et FXAA coupés), `upscale_override` (bancs), `--upscale=` (captures).
- [x] Réglage joueur `video/upscale` (auto / off / quality / performance) + menu Réglages > Affichage.
- [x] Bancs : `release_journey --ab-configs=off,metalfx_s:0.75,metalfx_t:0.75,…` ;
  bataille `--bench-ab=off,metalfx_s:0.75,…`. Test `tests/pb3b_upscale_test.gd`.
- [~] Mesures carte 1920×1080 `--uncapped` : série 1 faite ; les séries sont souvent plafonnées
  par le compositeur (6,9 / 16,7 ms : fenêtre masquée par d'autres agents) → script qui relance
  jusqu'à 3 séries valides (scratchpad `map_ab2.py`). Tendance : spatial 0,75 ≈ −15 à −30 %,
  temporel 0,75 ≈ −15 à −35 %, spatial 0,5 ≈ −25 à −45 %.
- [ ] Bataille, captures `docs/img/pb3b/`, choix des défauts, ADR 0080.

## Prochaine étape
Fin des mesures carte, puis bataille et captures.
