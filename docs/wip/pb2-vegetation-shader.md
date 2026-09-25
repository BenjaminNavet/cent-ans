# PB2 — Semis natif de la végétation + shader du terrain

Demande (2026-09-25) : pistes 1 et 2 de PB1 uniquement : (1) porter le semis des tuiles de
végétation en Rust, (2) alléger le shader du terrain de près sans changement visible.
Worktree `/Users/jean_hubert/dev/gp-pb2`, branche `pb2-veg-shader`. Coût cloud : 0 $.

## 1. Semis natif — fait (ADR 0062)
- Crate pure `core/crates/vegetation` (semis, haies, empaquetage, recalage ; opt-level 3 en dev).
- `VegetationScatter` (godot-bridge) : fils natifs, `set_map`, `request`, `request_reground`,
  `poll`. `Vegetation` : grille grossière en GDScript hors fil (`coarse_only`), puis requête
  native ; recalages natifs ; repli GDScript sans l'extension ou avec `--no-native-vegetation`.
- Mesures (M4 Pro, machine partagée, charge 20-85) :
  - `pb1_veg_job.gd` : semis+haies+empaquetage 124-327 ms → 2-10 ms par tuile ; grille
    grossière inchangée (25-35 ms, hors fil). Nombres par essence à ±1-3 % (flux aléatoire
    différent, mêmes lois).
  - Démarrage à chaud (5 tuiles) : 1 008 → 158 ms (Orléans), 920 → 125 ms (Angers).
  - `pb1_bench.gd` A/B dans la même branche : vue initiale complète 11,8 / 14,1 s → 6,5 / 6,4 s
    (≈ chargement 4,2 s + relief fin) ; arrivée au zoom 491 357 / 453 → 88 / 311 ms.
  - Recalage d'une tuile : 438 → 6 ms au pire ; `settlements_render_test` tree_error 0,0.
  - Captures A/B (Orléans d=40, Angers d=25) : même aspect (densité, essences, haies, pose).

## 2. Shader du terrain — fait (gains exacts seulement)
- Mesure par bloc (Vulkan, sonde temporaire `pb_skip` : uniforme global qui coupe un bloc, jamais
  commitée), d=40, base ~22,5 ms GPU : météo au sol `weather_ground` 1,3-1,5 ms, parcellaire
  ZG5b 1,3, textures de détail 1,2, parcellaire V2b 1,0, fleuves + zones humides 0,7, `fp_splat`
  0,7, frontières 0,3, terroirs 0,3, ombres de nuages ~0 ; tous les blocs coupés : −6,6 ms.
  Le reste (~16 ms) : géométrie (7,4 M primitives, surtout la végétation), ombres, MSAA.
- Fait (résultat identique, moins de lectures de texture) :
  - fragment : sur une page du quadtree (`qt_has_page`), plus de relief heightmap ni de
    `rl_relief` (9 lectures par pixel), remplacés de toute façon par `qt_relief` ;
  - vertex `qt_vertex` : hauteur grossière et fond de vallée ZG8 lus une fois quand le sommet
    ne glisse pas (`gm == g`) — 8 lectures de moins par sommet, dans les 6 passes (prépasse,
    couleur, 4 cascades d'ombre). `campaign_display_height_at_floor` (même expression).
  - Gain mesuré −0,3 à −0,6 ms (machine très chargée : 24 autres Godot, écarts de ±5 ms entre
    passes ; seul le signe est sûr). Captures A/B : écarts au niveau du bruit des animations.
- Écarté : espacement des sommets du quadtree (`qt_px` 6/8) — le relief ne fait qu'une petite
  part des primitives, aucun gain net.
- Au-delà, il faudrait accepter un changement (même faible) : parcellaire V2b coupé de près
  (encore mélangé à 8 % sous ZG5b), déformation météo précalculée en texture, Voronoï 2×2.
  Non fait (consigne : sans changement visible).
