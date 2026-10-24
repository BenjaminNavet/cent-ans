# 0787 — Sol de campagne : fin du patchwork de parcelles à bords nets

Date : 2026-10-24 · Lot TERRAIN2 (constats 2, 3 et « Ressenti » du juge DA, itération 4)

## Contexte

Le juge décrit en vue basse un sol en « grands rectangles aux bords nets » de verts différents (menthe
contre vert sombre), « tuiles de LOD ou de masque », et une campagne plus terne et bleutée que la bataille.

## Cause établie (bissection, Paris 2,35 / 48,85, d = 60, `CENT_ANS_WEATHER=clear`)

- Pas le cache de sol par nœud (ADR 0742) : `CENT_ANS_GROUND_CACHE=0` donne la même image ; ni les dalles
  de LOD, ni les masques de biome, ni `fp_parcels` (`tnofp` : inchangé).
- Les parcelles elles-mêmes, trop contrastées : `PERF_OFF=tnogdk` (`gd_parcel_k`) retire presque tout le
  damier, `tnofields` retire tout. Le lot TERRAIN (23/10) avait relevé `ground_detail.json` (contraste de
  luminance 0,6, saturation 0,45, teintes de stade verte 0,84/1,2/0,76, or 1,28/1,14/0,7) et
  `ground_biome_mix.json` (`patch_gain` 0,85, `patch_sat` 1,35 : la parcelle prend la couleur de sa
  culture) : les facettes de Voronoi, à peine gauchies et au flou de bord (35 m) inférieur au pixel
  (15 à 300 m), se lisent comme des rectangles.

## Décision

Données seules + deux retouches de `ground_detail.wesl` (aucun échantillonneur, aucun coût mesurable) :

- `ground_detail.json` : contraste inter-parcelles / saturation / amplitude des stades divisés
  (0,18 / 0,10 / 0,5), teintes de stade plus proches du neutre et plus chaudes (vert 0,97/1,07/0,90),
  flou de bord 160 m, variation continue dans la parcelle plus forte et plus courte (0,12, 300 m).
- `ground_biome_mix.json` : `patch_gain` 0,5, `patch_sat` 1,1.
- `gd_warp` : troisième octave fine (frange des facettes) ; `gd_edges` : largeur de la rampe douce modulée
  par un bruit basse fréquence (front irrégulier) et jamais inférieure à 3 pixels.
- Palette globale (`palette.json`, atmosphère, ADR 0755) non touchée.

## Conséquences

Parcelles à contour effrangé et teinte continue, passage doux entre cultures, plus de grands aplats nets
à d = 60, 200 et sur la côte d'hiver. Bench pan d30 (offscreen, charge machine 8 à 13) : avant 10,72 / 16,48 ms
(p50 / p95), après 10,77 / 16,93 ; une première paire à 10,08 / 19,18 (bruit). `shader_check` vert.
Limite : les facettes restent polygonales de près (les textures de cultures diffèrent) ; un mélange
vrai entre parcelles voisines demanderait un second tirage de Voronoi par pixel (coût), non retenu.
