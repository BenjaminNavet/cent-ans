# vq-cperf : marge de temps d'image sur la carte

## État
- Énigme VisibilityRange : hypothèse (voir bas) ; `fade_maquettes` ne touche plus les plages hors bande de fondu
  (interrupteur ancien chemin : `CENT_ANS_PERF_OFF=fadeband`).
- A faire : mesures A/B, étalement des rafales de spawn en descente.

## Analyse Bevy 0.20
- `RenderVisibilityRanges::range_to_index` n'est jamais purgée (65 535 emplacements, puis tout nouveau partage l'emplacement 0) :
  réécrire des plages continues chaque image fuit et finit par fausser le culling GPU. Les décalages sont donc quantifiés (1 unité).
- Une plage non abrupte (start != end) active la variante de pipeline VISIBILITY_RANGE_DITHER ; hors bande, plage abrupte constante.
