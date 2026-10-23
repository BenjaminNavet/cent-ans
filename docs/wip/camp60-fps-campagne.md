# camp60 : 60 fps sur la carte de campagne

Mandat du 22/10 (23 h) : monter la carte de campagne à 60 fps (16,7 ms par image), autonomie complète pour la nuit.
Branche `perf/camp60`, worktree `../gp-camp60` (données ignorées liées vers le dépôt principal).

## Référence
- `out/bench/map_mem-map.json` (22/10 10 h 40, machine chargée) : panoramique d = 30 p50 41 ms / p95 79 ms (~24 fps),
  descente p50 35 ms ; 573 meshes visibles (p50), 34 316 entités.
- Seuil `perf_guard` actuel : panoramique p50 20 ms.

## Mesures (panoramique d = 30, p50 ms ; machine peu chargée)
Conditions du joueur : `CENT_ANS_VIEW_SIZE=3456x2234` + copie de son `settings.json` (`CENT_ANS_SETTINGS_PATH`, Ultra, plein écran).
- Natif 3456×2234 : 101 ms (10 fps). Avec la mise à l'échelle auto (ADR 0740, 0,52) : 55 ms.
- Goulot = GPU (profil : `submit_pending_command_buffers` 26 ms sur le fil de rendu ; Main 11 ms).
- Bissection à l'échelle 0,52 : sans sol `tflat` 21 ; sans sol régional `tnoproc` 29 ; sans champs `tnofields` 39 ;
  sans ombres 42 ; sans ombres d'arbres 48 ; sans arbres 50 ; post 54. Eau, provinces, villes, vie, armées, marqueurs : < 1 ms.
- Outil : interrupteurs du shader du sol `CENT_ANS_PERF_OFF=t...` (liste `PERF_DEFS`, `terrain/material.rs`), `tshadow`.

## État
- 23/10 1 h 30 : SMAA au lieu du MSAA sur la carte (ADR 0741, 1be8b2d71) : Ultra joueur 55-62 → 45,8 ms.
  Trace Metal (xctrace) : passe opaque ≈ 35 ms de fragment (shader du sol), 4 cascades d'ombre ≈ 13 ms,
  pas de passe fantôme. Densité du relief (RELIEF_PX 6 → 24) : ~0 ms sur le shader plat.
  En cours : leviers d'ombres (`CENT_ANS_SHADOW_SIZE`, `CENT_ANS_MAP_CASCADES`).
- 23/10 0 h 40 : mise à l'échelle portée (ADR 0740, commit dc8361581). Bissection fine du shader du sol en cours.
- 22/10 23 h 15 : en attente, à la demande du joueur, de la fin d'une autre session (game-project-79) qui fait de gros changements.

- Partage avec game-project-79 (chantier Bevy/rendu) : elle cède la perf campagne. Ne pas toucher `campaign/terrain/`
  avant la fusion de son correctif des samplers : sans lui, la campagne plante au premier rendu sur Metal (17 samplers > 16).
  Prévenir avant chaque banc release de campagne.
- Plan : audit `git show worktree-agent-a37447ae40299d827:docs/wip/bevy-audit-campagne.md`. L1 et L2 sont déjà dans main ;
  les lots L3 à L7 reviennent à camp60 (L5 relief = `campaign/terrain/`, après la fusion de game-project-79).
- Piège : lier `data/map/pyramid` vers `~/dev/cent-ans-raw/compose/pyramid` et faire
  `rsync -a --ignore-existing <principal>/game/assets/ game/assets/` ; tuer le client par son pid ; `CARGO_INCREMENTAL=0`.

## Prochaine étape
Cache du sol **fait** (939c58718, ADR 0742, préréglages Ultra 320 / Haute-Moyenne 256 / Basse 192) :
d=30 47,5 → 27-28 ms, d=150 33,9 → 14,6, d=500 34,1 → 13,8 (réglages du joueur).
Reste : d=30 (≈ 27 ms). Bissection avec cache en cours (`c_*`) : ombres (arbres surtout),
végétation, reste du fragment (roche, neige, côte, météo, nuages).
Puis : capture de contrôle, fusion dans main (worktree dédié, ff-only), ADR 0740/0741/0742 dans INDEX.
Réglages de banc ajoutés : `CENT_ANS_GROUND_CACHE=<texels>`, `GC_FIRST`, `GC_REFRESH`, `GC_STRESS`,
`GC_LOG` (journal des cuissons), `VEG_CASCADES`.
