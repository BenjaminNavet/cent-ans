# Lot B1 — Maillages des soldats et des chevaux (branche `worktree-agent-a1fa8fd1c52def061`)

Plan : `docs/design/2026-09-24-rapprochement-total-war.md` (lot B1).

## Approche retenue
- Modélisation en script Blender reproductible : `tools/blender/battle_figures.py`
  (`blender --background --python tools/blender/battle_figures.py`), qui exporte un `.glb` par
  figure et par niveau de détail sous `game/assets/models/battle/` et un fichier de pivots
  `figures.json`.
- Formes par « lofts » (sections elliptiques le long d'un chemin) lissées par subdivision
  Catmull-Clark : torse, membres, cheval (poitrail, ventre, croupe, encolure, tête), caparaçon.
- Codage dans le glb : COLOR_0 = (couleur linéaire, code matière / 5), TEXCOORD_0 = UV blason,
  TEXCOORD_1 = (membre, poids de flexion du genou/jarret).
- `BattleMeshes.soldier()` charge le glb et reconstruit un `ArrayMesh` au format du shader
  (CUSTOM0 = membre + pivot + poids de flexion ; UV2 = pivot du genou), mis en cache.

## Pourquoi Blender plutôt qu'un SurfaceTool plus détaillé
- Subdivision Catmull-Clark, solidification des étoffes, recalcul des normales et lissage par
  angle gratuits : formes organiques (encolure, croupe, jarret) impossibles à obtenir proprement
  avec des cylindres, sans code de maillage à maintenir côté jeu.
- Les niveaux de détail sortent du même script (subdivision coupée, anneaux et sections décimés,
  détails retirés), sans coût de génération au chargement de la bataille.
- Le script est la source (pas de `.blend`) : relancer `blender --background --python
  tools/blender/battle_figures.py` puis `godot --headless --path game --import`.
- Repli conservé : sans glb (ou avec `--legacy-figures`), l'ancien procédural V4 est utilisé ;
  les engins de siège et leurs servants restent procéduraux.

## État : terminé (non fusionné)
- [x] Script Blender (`tools/blender/battle_figures.py`) : 9 figures × 3 niveaux (complet subdivisé,
  moyen, lointain/ombres), `figures.json` (pivots, genoux) ; aperçu `tools/blender/preview_figures.py`
  (`FIGURES_DEBUG=1` imprime les triangles par pièce)
- [x] Chargement et conversion dans `BattleMeshes.soldier_level()` (cache), repli procédural V4
  (`--legacy-figures` après `--` force l'ancien rendu pour les comparaisons)
- [x] Shader : flexion genou/jarret (CUSTOM0.w, pivot UV2), pas du cheval en quatre temps,
  galop avec antérieurs repliés, jambe arrière soulagée au repos, blason hors UV = livrée
  (caparaçon armorié sur les flancs, crinière de tissu toujours teinte)
- [x] `BattleSoldiers` : complet < 32 m, moyen < 75 m, lointain au-delà et pour les ombres
- [x] Banc d'essai : `--camera=` appliqué aussi à `--benchmark` (mesure en gros plan)
- [x] Captures `docs/img/b1/` : `before_|after_` × `knights`, `archers` (hors simulation,
  `game/tests/b1_figures_shot.gd`), `battle_closeup`, `battle_overview` (bataille réelle, 300 s)
- [x] Smoke vert ; ADR `docs/decisions/0006-battle-figures-from-blender.md`

## Triangles (complet / moyen / lointain ; V4 : complet / allégé)
| Figure | B1 | V4 |
|---|---|---|
| Fantassins | 3 750-4 700 / 900-985 / 255-290 | 410-460 / 190-220 |
| Archers, arbalétriers | 3 850-4 100 / 910-1 060 / 255-335 | 440-475 / 215-230 |
| Cavaliers | 7 700-9 900 / 1 700-2 050 / 525-695 | 965-1 055 / 405-460 |

## Mesures (Apple M4 Pro, `--disable-vsync`, 1600×900, 600 images, A/B `--legacy-figures`
sur le même binaire, passes alternées ; machine très chargée par d'autres agents : ±30 %)
| Config | V4 (legacy) médianes i/s | B1 médianes i/s | Primitives |
|---|---|---|---|
| `--units=20` (4 800 soldats, vue lointaine) | 47,1 · 53,3 · 38,6 · 47,5 (moy. 46,6) | 50,3 · 34,6 · 54,4 · 36,0¹ (moy. 43,8) | 1,64 M → 1,90 M (+16 %) |
| `--camera=630,240,28,180` (1 160 soldats, gros plan chevaliers) | 53,3 · 56,3 · 37,9 · 48,0 (moy. 48,9) | 55,4 · 44,7 · 50,0 · 36,0¹ (moy. 46,5) | 1,81 M → 2,61 M (+44 %) |

¹ première passe d'une série (préchauffage) : systématiquement plus lente, quel que soit le rendu.
Écart moyen ≈ −5 % (dans le bruit), sous le seuil de 15 %. Appels de dessin inchangés (±8).

## Points ouverts
- Animation héritée de V4 : l'archer au tir lève le bras gauche avec l'arc, mais la main droite
  ne vient pas à la corde (le recul `draw` est linéaire) ; pas de coude ni de torsion du buste.
- Cavaliers : la jambe du cavalier est figée (P_RIDER) ; les rênes ne suivent pas les mains.
- Le caparaçon peut être traversé par les antérieurs au galop (tissu rigide).
- Servants d'engins encore procéduraux (V4).
- Captures de bataille : la démo ne va toujours pas au contact en 300 s (cf. V4b) ; gros plan de
  la charge des chevaliers seulement.
