# VIS-R3a : carte de couleur rebâtie

Spec : `docs/audits/2026-10-29-visuel-campagne.md` (g) Lot R3, sous-lot R3a ; palette (c). Branche de travail : worktree, pas de fusion.

## État
- [x] squelette (cette note)
- [x] palette `colormap_style.yaml` : ordre de valeur, `crop_contrast` 0,85, routes (0,55 0,48 0,36)
- [x] classes régionales (section `regional` du style, `regional_masks`) : bocage, beauce, meseta, landes
- [x] noyaux villageois (`villages`, `_paint_villages`)
- [x] tests (hiérarchie de valeur, classes régionales, village)
- [ ] régénération `geo colormap`, mesures avant/après
- [ ] bible (c) : teintes ajustées pour l'ordre de valeur

## Prochaine étape
Régénérer (tools/heavy.sh), mesurer avant/après, ajuster la bible (c), livraison.

## Points ouverts
- La carte de couleur (`data/map/colormap_bc1_*.bin`) est suivie par git, pas dans le paquet de relief (ADR 0149) : voir la section livraison à la fin.
