# XXXX — VIS-R5 : eau de la carte (bathymétrie réelle, rivières)

Statut : accepté (lot VIS-R5, 30/10/2026). Numéro à fixer à la fusion.

## Contexte

Audit visuel de la campagne (`docs/audits/2026-10-29-visuel-campagne.md`, G2-2, G2-8, ligne R5) : la profondeur de la mer
était estimée depuis la distance à la côte (4,5 m par pixel, ADR 0750), donc sans bathymétrie réelle : large uniforme
ardoise, bande peu profonde de quinze pixels, liseré cyan (`turquoise_saturation` 0,62 sur fond gris) ; fleuves sans berge.

## Décision

1. **Bathymétrie cuite** : `cent-ans geo bathymetry` (`tools/cent_ans_tools/geo/bathymetry.py`) lit GEBCO 2025 (hors dépôt,
   `~/dev/cent-ans-raw/gebco/`, lignes 27-67 N téléchargées en plages d'octets), réduit à 1' (moyenne des cellules de mer),
   relit la profondeur au point réel de chaque texel composé (`ComposedWarp.inverse`), remplit la terre réelle par la mer la
   plus proche, et écrit `data/map/bathymetry.png` (L8, 1770 x 1526 = 1/4 de la carte, 0,3 Mo, versionné comme
   la carte de couleur, ADR 0142) et `bathymetry.json` (schéma `bathymetry.schema.json`). Codage `255 * sqrt(profondeur / 4000 m)`.
2. **Mer** (`sea.wesl`) : liaison 13 (texture seule, échantillonneur de `coast_dist` réutilisé : aucun échantillonneur de plus).
   Profondeur = min(bathymétrie, `shore_m_per_px` x distance à la côte composée) : la côte reste à 0 m. Rampe de couleur
   sable-vert / plateau (50 m) / talus (200 m) / grand fond (`abyss_color`, 2 500 m). Tous les réglages sont dans
   `data/fx/sea_life.json` bloc `eau.bathymetry` (schéma `fx_sea_life`). Liseré cyan retiré (saturation 0,35, `shallows`
   10 m, couleur sable) ; vase d'estran éclaircie. Interrupteur de comparaison `CENT_ANS_BATHY=0`.
3. **Rivières** : plancher de largeur du palier moyen `major_min_px` 4 -> 6 avec échelle par importance 0,5 à 1,33 (3 à 8 px) ;
   palier fin `fine.min_px` 6 (échelle 0,3 à 1,33) ; bande hors de l'eau en trois zones (berge claire, ripisylve sombre,
   plaine alluviale verte), `look.riparian` + `wet_band_px` 12, dans `river.wesl` (palier moyen seulement).

## Conséquences

- Source : GEBCO 2025 Grid, usage libre avec attribution (ajouté à `CREDITS.md`). Écart à la spec : aucun ; la grille de
  l'écran de crédits n'est pas modifiée.
- Raster 0,3 Mo : pas de paquet de relief.
- Coût GPU : une lecture bilinéaire de plus par pixel de mer.
- Limites : les rivières n'apparaissent pas à d 600 dans la capture de contrôle (avant comme après : voir `docs/wip/vis-r5.md`) ;
  la Gironde reste un coin (forme du masque de terre composé, hors de ce lot).
