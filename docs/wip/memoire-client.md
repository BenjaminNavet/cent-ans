# Lot MEM : mémoire du client Bevy

## État
- Sonde `memory` (dump d'état + rapports de banc), garde RSS dans `perf_guard.sh`, doc `docs/performance-memoire.md`,
  ADR 0748. Essais relief du tas et cache disque abandonnés (sans gain d'empreinte).

## Prochaine étape (machine calme)
- Mesurer `footprint` avant/après : copies CPU en `RENDER_WORLD` (castle_wall, building arrays, flipbook, battle_ga3),
  décodage des feuilles de sol en bandes, déchargement campagne -> bataille.
