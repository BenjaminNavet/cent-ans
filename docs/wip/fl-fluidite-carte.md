# FL — fluidité de la carte : arbres qui clignotent, appels de dessin, FPS (2026-10-08)

Demande du joueur : « les trois me gênent » (arbres qui clignotent, appels de dessin, FPS de la
carte), après avoir écarté une migration Unity/Unreal. Branche `feat/fl`, worktree `../gp-fl`
(dylib copiée de main, `data/map/pyramid` en lien symbolique). Coût cloud : 0 $.
Reprend `docs/wip/arbres-clignotants.md` et `docs/wip/fps-carte.md` (PF, ADR 0169).

## État
- [x] FL0 banc de référence (main d327f2ebd, Haute auto, fenêtre Retina ; charge ≈ 2 au début,
  25 à la fin : la session SC a démarré pendant d = 150).
  | d | image p50 | p99 | i/s | appels p50 | primitives p50 |
  |---|---|---|---|---|---|
  | 30 | 31,2 ms | 100 ms | 27,9 | 652 | 2,87 M |
  | 150 | 45,7 ms | 116 ms | 22,2 | 865 | 3,06 M |
  Sondes (moyenne par image / pics dominés) : **`map.misc` 7,8 ms (d 30) et 12,1 ms (d 150), en tête de
  316 et 391 pics** ; `settle/declutter` 3,9 / 6,2 ms ; `map.update_lod` 2,2 / 2,6 ms ; non attribué
  4,7 / 8,6 ms ; `life/reground` pics à 35 ms. Descente : 249 des 253 pics dominés par les scripts.
  → Le FPS est d'abord limité par le **GDScript du fil principal**, pas par le GPU.
- [x] FL1 arbres qui clignotent : ombres des arbres allumées pour toute la vue selon le zoom
  (`_tree_shadows_on`, hystérésis 8 %), plus partie par partie. Sonde `tree_flicker_probe --keys`
  (bascules d'ombre de parties visibles aux deux images) : rig 60 **22 → 0**, rig 40 **30 → 0**.
  Tests ga3_l2_vegetation, hc_forest, fc1_shadows OK ; tf_far_layer échoue sur un seuil de temps
  (124 ms > 8, charge 231 : SC). Coût des ombres en plus à rig < 64 : à mesurer en A/B (FL3).
  Restes possibles du clignotement : changements de niveau du relief (64 en 150 images) et
  recalages d'arbres une image après (≤ 22 % de la hauteur) — à revoir si le joueur en voit encore.
- [ ] FL2 appels de dessin et primitives.
- [ ] FL3 images p50/p99 de la carte (shader du terrain, pics de scripts).
  - Base FL0 prise à froid (cache) : peu fiable. Nouvelle base à chaud avec curseur simulé au
    centre (`--bench-hover`) : d 150 p50 18,9 / p99 41 ms ; d 30 p50 19,8 / p99 41 ms ;
    `misc/object_hover` 3,4 / 2,4 ms en moyenne (`map.misc` découpé en 5 sondes).
  - FL3a en cours : `SettlementCells` (grille 256 u, sphère par case) ; dé-encombrement limité aux
    cases du champ élargi (+ colonies encore affichées) ; survol limité aux cases proches du rayon.
    Tests : sa_pick OK, smoke OK. **Préexistants, sans lien avec FL** (identiques avec le fichier
    d'origine) : tb2_declutter « minor place … should lose its shield at distance 400 » (11-14
    lieux, règle `_shield_until` TB2) ; settlements_render_test se bloque au chargement (fil
    principal sur une variable de condition du moteur, 0 % CPU).

## FL1 — diagnostic (lecture du code)
Style généralisé (HC1) + imposteurs GA3 actifs par défaut (`Ga3Vegetation.near_impostors()`) :
`_meshes(DETAILED)` et `_meshes(FAR)` rendent les **mêmes** imposteurs ; le changement de palier
par partie de tuile (`lod_d` < `generalised_mesh_distance` = 60) ne change donc que
`cast_shadow` de la partie entière. En panoramique au zoom moyen (rig < `generalised_shadow_distance`
= 70), les parties franchissent cette frontière en continu : l'ombre de tout un paquet d'arbres
apparaît ou disparaît d'une image à l'autre. C'était la piste 1 de `arbres-clignotants.md`
(4 bascules d'ombre en 150 images à rig 60). `ForestDetail` n'est pas créé en style généralisé.

## Coordination
Session SC (`../gp-sc`, `feat/sc`, simplification de tout le code, 20 agents) démarrée le 08/10 :
elle peut toucher les fichiers FL. Bancs FL : uniquement `--bench-ab` en processus (charge).

## FL3 — mesures du 08/10 (fenêtre en arrière-plan, `tools/godot_bg.sh`, A/B en processus)
- La sonde `--bench-probe` gonfle les images (≈ 29 ms contre 14 sans) : p50/p99 sans sonde.
- **Bimodalité des bancs = taille de fenêtre** : `settings.cfg` du joueur a `fullscreen=true` ; le
  banc s'ouvre tantôt en 1440×900 (échelle 0,75 → 0,73 M px : **16-20 ms**), tantôt plein écran sur
  le Dell HiDPI 2× (4096×2304, budget ADR 0123 au plancher 0,5 → 2,36 M px : **38-43 ms**).
  Le joueur joue en plein écran : c'est son cas réel.
- Grille FL3a : sans effet sur la médiane (14,1 / 14,0 ms), coupe le survol (3,4 → 0,7 ms) et ses pics.
- Pixel-bound : échelle 0,25 → 17 ms ; 0,42 → 30 ; 0,35 → 25 (contre 38-40). Ombres du soleil −1,2 à −3.
- Terrain = moitié de l'image (hide:Terrain 15,9 → 8,1 en fenêtre). Aucun effet ne domine :
  météo −2 à −6,5 ms, vie −2 à −2,8, brouillard −1 ; reliefs, côtes, biomes, satellite ≈ 0.
- Météo : 205/443 provinces actives (84 % avec voisines) en automne 1337 : un saut « loin de
  toute météo » n'aide pas (essayé, annulé). Piste : cuire la météo en texture carte basse
  résolution à chaque tour (frange comprise) → 1 lecture au lieu de 2 fbm + 3 lectures.
- Bancs : `--bench-set/--bench-ab=prop:nœud.propriété=valeur` (ex. `prop:settlement_layer.use_cells=false`).

- **Plancher 0,4** (choix du joueur, ADR 0191, 2a351e6af) : plein écran Dell 4096×2304 **38-43 → 26 ms**.
- **Météo cuite par tour** (ADR 0192, 598edcf83) : −0,8 ms en A/B (26,3 → 25,5) ; rendu identique au
  bruit de capture près (`tests/fl_weather_field_shot.gd`, effets forcés).
- **Pics de bascule de niveau** : écouteurs de `chunk_surface_changed` chronométrés un à un ; seuls
  les ponts (`Crossings`, 20 ms : maillage bâti sur le fil principal) comptent → construction étalée à
  l'image suivante (6c26a1901). Pic Landmark_rouen 121 ms non reproduit (charge machine).
  `life/reground` ≤ 10 ms (déjà découpé, RS-K2).

## Prochaine étape
FL2 appels de dessin : `a6_drawcalls_probe` (ablation par couche, d 150/60) en cours ; s'attaquer aux
une ou deux couches dominantes. Puis fusion dans main (worktree dédié, --ff-only), mémoire FL.
