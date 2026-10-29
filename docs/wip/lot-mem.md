# Lot MEM (boucle juge, itération 1) : mémoire de campagne, fps, garde

## État
- `perf_guard.sh` réparé (`status` est réservé en zsh, renommé `rc`).
- Branche remise à niveau sur main (elle avait 317 commits de retard) ; cherry-pick des outils MEM3 (memlog, allocation
  Metal, protocole `mem_protocol.sh`, `gpu_pacing`).
- Cause de la régression de campagne : pic transitoire des textures RGBA8 2048 des modèles `dn` (4,5 Go
  d'IOAccelerator au chargement) puis, en régime établi, ~1,3 Go de ces textures gardées en vie (l'échange en place de
  l'image vers BC1 n'atteint jamais les bind groups des matériaux ; Metal alloué 2 000 Mo contre 1 030 sans elles).
- Correctif : capture des pixels à l'`Added` (marqueur 4x4 de la couleur moyenne), puis BC1 dans un NOUVEL asset image
  et re-pointage des matériaux (`client/src/fx/model_mips.rs`).
- Worktree : lier `data/map/pyramid` -> `~/dev/cent-ans-raw/compose/pyramid` (sinon la campagne se mesure sans relief),
  copier `game/assets` et `out/ktx2_cache` depuis le checkout principal.

## Mesures (campagne idle, release, hors écran, charge 15-40, Mo d'empreinte physique)
| Binaire | t+10 s | t+30..60 s |
|---|---|---|
| base (main + outils), avec relief | 5 214 | 6 188-6 224 |
| marqueur sans re-pointage (cassé : modèles gris) | 5 394 | 5 399 |
| nouvel asset + re-pointage (v3) | à mesurer | à mesurer |

## Prochaine étape
Voir les sections suivantes (mises à jour à chaque commit).
