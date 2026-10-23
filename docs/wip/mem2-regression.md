# MEM2 : régression de mémoire en campagne

Constat du juge (iter-03, R2) : footprint campagne 10-12 Go (budget 3 500 Mo, ADR 0748).

## Cause (mesurée, build release, main c26909646, Ultra, offscreen, machine chargée 30-80)
- `models/dn/*` (33 modèles de lieux de campagne, ADR 0211) : 66 textures 2048x2048 RGBA8 décodées par
  le chargeur glTF (MAIN|RENDER_WORLD), puis mips CPU par `fx/model_mips.rs` : 21 Mo GPU + 21 Mo RAM
  chacune = 1 276 Mo GPU + 1 276 Mo RAM, plus ~1 Go de blocs libérés (« MALLOC_LARGE (empty) »).
- Le lot responsable n'est pas un des 10 lots récents mais l'arrivée des modèles dn (lot « towns »,
  TS-T4/T7) combinée au budget jamais retenu en footprint ; pas de bissection par checkout (build release
  8 min x N sous charge 50-90 : inabordable). Le diagnostic vient du dump `textures.by_prefix`.
- Le worktree d'un agent n'a pas les assets non suivis (`game/assets/models/dn`, `out/ktx2_cache`) :
  les copier depuis le checkout principal (`rsync -a --ignore-existing`) avant toute mesure.

## Corrections
1. `fx/model_mips.rs` : albédos opaques de `models/dn/` et `models/props_ga/` : chaîne de mips + BC1 sur
   un fil de calcul, `RENDER_WORLD` seul (plus de copie CPU).
2. `RENDER_WORLD` après conversion : pierre de siège (`stone.rs`), bandes d'atlas (`strip_to_array`),
   flipbooks (`ImageLoaderSettings.asset_usage`).
3. `footprint_mib` dans le dump `memory` et les rapports de banc ; `perf_guard.sh` contrôle footprint et
   pic RSS (seuils `perf_thresholds.json`).
4. `CENT_ANS_RELIEF=<s>` (expérience) : `malloc_zone_pressure_relief` ne rend rien (0 Mo), confirme l'ADR 0748.

## Mesures (footprint, Mo)
| Scénario | avant | après |
|---|---|---|
| Campagne t+8 / t+30 | 6 763 / 6 549 | 4 713 / 4 878 puis 4 298 / 4 299 (même binaire, bruit de charge ±500) |
| Menu | 882 | - |
| Bataille (Crécy) t+8 / t+30 | 3 493 / 3 619 | (à refaire) |

## Prochaine étape
- Reconstruire, mesurer campagne/bataille, ADR 0776, docs/performance-memoire.md.
