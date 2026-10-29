# VIS-R3a : carte de couleur rebâtie

Spec : `docs/audits/2026-10-29-visuel-campagne.md` (g) Lot R3, sous-lot R3a ; palette (c). Branche de travail : worktree, pas de fusion.

## État
- [ ] squelette (cette note)
- [ ] palette `colormap_style.yaml` : ordre de valeur forêt < pré < labour < chaume, `crop_contrast` relevé, routes
- [ ] classes régionales (Beauce, bocage, Meseta, Landes) via `agri_landscapes.json` / `agri_regions.png`
- [ ] noyaux villageois (jardins, vergers) autour des lieux indexés
- [ ] tests (`tools/tests/test_colormap.py`) : ordre des luminances de classe
- [ ] régénération `geo colormap`, mesures avant/après
- [ ] bible (c) : teintes ajustées pour l'ordre de valeur

## Prochaine étape
Palette puis classes régionales.

## Points ouverts
- La carte de couleur (`data/map/colormap_bc1_*.bin`) est suivie par git, pas dans le paquet de relief (ADR 0149) : voir la section livraison à la fin.
