# PB1 — Benchmark et optimisation des performances (FPS, chargements, zooms)

Demande (2026-09-25) : benchmarker le jeu et optimiser FPS + temps d'exécution (chargements,
zooms) sans dégrader le contenu. **État : fusionné dans main** (f2eacfb1 ombres, puis commit
`perf(map): PB1 …`). ADR 0051. Coût cloud : 0 $.

## Outils de mesure
- `godot --path game --script res://tests/pb1_bench.gd [-- --views=1500,491,150,40 --trace --veg-jobs=N]`
  → `PB1_JSON` : chargement carte, stabilisation après chaque saut de zoom (relief + végétation),
  pire image pendant la stabilisation, temps d'image par vue (médiane de 3 échantillons), zoom
  animé France ↔ Paris (médiane / p95 / max). `--trace` : pics > 60 ms attribués aux écouteurs de
  `chunk_surface_changed`. GPU seulement avec `--rendering-driver vulkan` (Metal ne mesure pas).
- `godot --path game --script res://tests/pb1_turns.gd` → `PB1_TURNS` : six fins de tour complètes.
- `godot --path game --rendering-driver vulkan -- --journey --uncapped --map-ab=<d> --ab-configs=…`
  A/B GPU. Configs combinables `angular:0+soft:3+no_blend`, `nocast:<nœud>`, `hide:<nœud>`.
  Bug corrigé : `no_fog`/`no_shadows` n'étaient jamais restaurés (toutes les « base » faussées).
- Parcours complet : `godot --path game -- --journey --uncapped` (`JOURNEY_JSON`).
- Méthode : machine partagée (charge moyenne 15-35 pendant la session) → base et PB1 alternées
  dans le même créneau, 3 tours, médianes.

## Résultats (M4 Pro, 1440×900, qualité Haute, Metal sauf mention)
Base = main d'avant PB1 (déjà avec ZG2/ZG3), PB1 = branche ; médianes de 3 tours alternés.

| Mesure | Base | PB1 |
|---|---|---|
| Chargement campagne (parcours) | 4,83 s | 3,84 s |
| Chargement carte (banc) | 4,98 s | 4,28 s |
| Premier tour (ouverture diplomatie) | 744 ms | 239 ms |
| Tours suivants | 344 ms | 276 ms |
| Fin de tour du parcours | 464 ms | 387 ms |
| Zoom animé p95 | 128 ms | 96 ms |
| Arrivée au zoom 150 (pire image) | 468 ms | 395 ms |

Après rebase sur ZG4 (recalages étalés, faits de leur côté) : zoom animé max 112 ms, p95 63 ms ;
pire image arrivée d=150 228 ms, d=491 49 ms, d=40 83 ms ; chargement 4,0 s ; tours 225-326 ms.

GPU (Vulkan) : soleil sans PCSS −4 à −6,5 ms aux zooms proches (23-29 → 19-22 ms avant ZG2).
Sur le relief quadtree : ~24 ms à d=150/40, dont relief ~12 ms (shader), ombres ~7,5, MSAA ~5,5.
Bataille (banc intégré) : plafond 60 i/s par la vsync, GPU 8,4-9,5 ms, CPU de rendu < 1 ms,
jusqu'à 28 800 soldats. Il reste de la marge.

## Fait
1. Soleil de campagne sans PCSS (`light_angular_distance` 0 ; `shadow_blur` garde le flou) —
   aucune différence visible (captures comparées).
2. `FrameBudget` (6 ms/image) : tuiles proches, rubans de route, hameaux (≥ 1 par image).
3. Recalages : moulins, cheminées et feux des seules tuiles modifiées, tampon MultiMesh écrit en
   bloc (repli point par point en headless). Étiquettes une fois par image (fait aussi par ZG4).
4. Tuiles proches : indices en cache, hauteurs sans `surface_get_arrays` ;
   `TerrainBuilder.surface_heights_at` (lot, identique à `surface_height_at`) pour les routes.
5. Fins de tour : textures de mini-carte en cache par `MapData` (−570 ms au premier tour),
   marqueurs d'armée gardés si l'armée est identique, masque de terroir hors fil, effets de vie
   sautés si les entrées effectives n'ont pas changé.
6. Chargement : six masques décodés en parallèle (582 → 208 ms), sommets des 256 tuiles
   lointaines en parallèle.

## PB1b (soir du 2026-09-25) : végétation au démarrage
- Constat : juste après le chargement, première image figée ~2 s (démarrage à chaud de la
  végétation qui attend 5 tuiles) puis ~13 s avant la vue initiale complète. Une tuile coûte
  ~300 ms (GPU libre), dont ~75 % pour les haies : 145 000 candidats pour ~1 750 buissons.
- `VegetationTileJob._scatter_hedges` : borne sûre `p_max` de `hedge_probability` par tuile ;
  un bord dont le tirage haché la dépasse est sauté sans calculer `to_map`. Tirages aléatoires
  après les tests déterministes : les bords plantés sont les mêmes, trouées et arbres de haie
  gardent leur loi. Nombre d'instances ±0,3 %.
- Mesures (meilleur de 3, machine très chargée) : semis 282 → 181 ms (forêt d'Orléans),
  329 → 299 (bocage), 303 → 237 (Île-de-France), 250 → 92 (nord). Démarrage : image figée
  2,0 → 1,35 s ; vue initiale complète 17,1 → 11,5 s depuis le début du chargement.
- Banc : `godot --headless --path game --script res://tests/pb1_veg_job.gd` (coût par étape).
- Correction du banc : `first_settle_ms` comptait deux fois l'attente.

## Pistes non traitées (par ordre de gain estimé)
- Végétation : `VegetationMask.sample` (~35-110 ms par tuile, un dictionnaire par point) ; le
  bocage garde des haies chères (`p_max` haut) → portage natif possible si besoin.
- Shader du terrain (~12 ms GPU de près) : partagé avec ZG/R1/CM2, en évolution. Profiler par
  bloc (parcellaire `field_at`, couches, côtes) avant de toucher.
- Premier `refresh_all` au chargement (~800 ms : croissance des colonies 384 ms, figurines).
- `settlement_layer.setup` (~630 ms) et `rivers.build` (~260 ms) au chargement (domaine ZG4).
- Végétation au zoom 491 : ~15 s pour la forêt complète (tâches ~1 s ; 10 en parallèle ne
  gagne que 2 s).
- Cœur Rust de fin de tour : 80-115 ms en profil `dev` (plus rapide en export release).
