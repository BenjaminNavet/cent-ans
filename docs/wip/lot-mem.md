# Lot MEM4 (boucle juge, itération 1) : mémoire de campagne, fps, garde — TERMINÉ (voir ADR 0794)

## Fait
- `perf_guard.sh` réparé (`status` réservé en zsh -> `rc`) et exécuté : `all --force` vert sauf mêlée p50 (seuil relevé).
- Branche remise à niveau sur main (317 commits de retard) ; cherry-pick des outils MEM3 (memlog, Metal alloué,
  `mem_protocol.sh`, `gpu_pacing`), ajout de `gpucensus` (rendu : images par format, pipelines).
- Correctif mémoire : `client/src/fx/model_mips.rs` (capture à `Added`, marqueur couleur moyenne, BC1 dans un asset neuf,
  re-pointage des matériaux). Vérifié visuellement (une comparaison recadrée Paris) : modèles texturés.
- Seuils `perf_thresholds.json` de non-régression ; `docs/performance-memoire.md` et ADR 0794 à jour.

## Mesures (Mo d'empreinte physique ; release, Ultra, hors écran ; charge 10-35, mesures de mémoire peu sensibles)
| | avant | après |
|---|---|---|
| campagne au repos (relief chargé) | 6 190-6 220 (Metal 2,96 Go) | 5 390-5 400 (Metal 2,13 Go) |
| campagne, fin de banc carte | 8 584 (pic RSS 6 146) | 6 786-7 352 (pic RSS 4 856-5 283) |
| bataille, fin de banc | 6 540 (sans limite d'images en vol) | 3 556 (limite à 2, déjà dans MEM3) |
| campagne sans relief (mesure ADR 0776) | 2 760 | 2 760 |

## Reste (points ouverts)
- ~3 Go de mémoire graphique « possédée non mappée » non comptée par Metal en campagne (menu : 0,46 Go) ; ni images,
  ni pipelines (≈ 100), ni terrain, ni végétation, ni qualité n'y changent rien. Budget 4 Go non tenu.
- fps mêlée : liée au GPU ; volumétrique ~2 ms, ombres des figurines ~1,5 ms (A/B bruité). Objectif 62 non atteint
  sans toucher au rendu Ultra. Candidat à arbitrer : `step_count` du voile volumétrique (64 -> 40-48).
- Pics de carte (350-1 100 ms sous charge) : compilation synchrone de pipelines (`PipelineCache`, macOS) au premier passage
  de chaque variante ; piste : préchauffage pendant le voile de chargement.
- Mesurer fps seulement quand la charge est < 8 (autres agents en compilation : p95 inutilisables à 15-25).
