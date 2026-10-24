# 0776 — Mémoire de campagne : textures de modèles GPU seules, BC1, mimalloc

Date : 2026-10-23. Statut : acceptée. Amende l'ADR 0748 (budget et métrique). Détails : `docs/performance-memoire.md`,
`docs/wip/mem2-regression.md`.

## Contexte
Le juge (iter-03, R2) mesurait 10-12 Go d'empreinte en campagne pour un budget de 3,5 Go. Reproduit à 6,5-6,8 Go
(machine chargée, release, Ultra, hors écran). Le dump `textures.by_prefix` désigne un seul poste : les 66 textures
2048x2048 RGBA8 des modèles de lieux `models/dn/*` (ADR 0211) : 21 Mo GPU et 21 Mo de copie CPU chacune (mips CPU de
`fx/model_mips.rs`), soit 1,3 Go + 1,3 Go, plus ~1 Go de tampons de décodage libérés mais gardés par libmalloc
(« MALLOC_LARGE (empty) », que `malloc_zone_pressure_relief` ne rend pas : 0 Mo mesurés). Les fusions récentes ne sont
pas en cause : la campagne chargeait déjà ces modèles ; la mesure de l'ADR 0748 avait été faite sans eux (assets non
suivis absents du worktree de mesure).

## Décision
1. Albédo opaque des modèles `models/dn/` et `models/props_ga/` : pixels pris au `Added` de l'image (avant le premier
   envoi au GPU, un marqueur 4x4 gris tient la place), réduits à 1024 px max, chaîne de mips + BC1 sur un fil de calcul,
   `RENDER_WORLD` seul. 21+21 Mo -> 0,7 Mo par texture ; plus de texture RGBA8 2048 créée sur le GPU. Un albédo
   translucide garde la chaîne RGBA8 (GPU seul).
2. `RENDER_WORLD` seul après conversion CPU pour la pierre de siège, les bandes d'atlas (`strip_to_array`) et les
   flipbooks.
3. Allocateur global mimalloc (feature `system-alloc` pour revenir à celui du système, mesures A/B) : rend les tampons
   libérés à l'OS ; supprime la variance de 1 Go de l'empreinte de campagne.
4. Métrique du budget : **empreinte physique** (`memory.footprint_mib` du dump et des rapports de banc, via
   `proc_pid_rusage`), plus le pic RSS à titre indicatif. Seuils (`perf_thresholds.json`, clé `memory`) :
   empreinte campagne 4 000 Mo (cible bataille 3 500 Mo, non tenue : le garde bataille est à 7 000 Mo, voir points
   ouverts) ; pic RSS campagne 4 500, bataille 4 000.

## Conséquences
- Campagne : empreinte ~2,8 Go (était 6,5-6,8), pic RSS ~3,6 Go (était ~5).
- Les textures de modèles dn apparaissent grises quelques dizaines de ms avant l'encodage.
- Un lecteur CPU futur de ces textures (`Assets<Image>::get().data`) trouverait `None` : lire la source disque.
- Bataille : l'empreinte oscille de 3,8 à ~6 Go selon `IOAccelerator (graphics)` (0,4 <-> 2,3 Go), indépendamment de
  l'allocateur et des textures ; cause non élucidée (point ouvert).
- Non fait : décharge des ressources de campagne à l'entrée en bataille (à mesurer sur le scénario campagne -> bataille),
  pré-cuisson KTX2 des feuilles de sol de campagne (décodage transitoire de 185 Mo par feuille, absorbé par mimalloc).
