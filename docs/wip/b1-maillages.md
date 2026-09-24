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

## État
- [x] Script Blender (`tools/blender/battle_figures.py`) : 9 figures × 3 niveaux (complet subdivisé,
  moyen, lointain/ombres), `figures.json` (pivots, genoux) ; aperçu `tools/blender/preview_figures.py`
- [x] Chargement et conversion dans `BattleMeshes.soldier_level()` (cache), repli procédural V4
  (`--legacy-figures` après `--` force l'ancien rendu pour les comparaisons)
- [x] Shader : flexion genou/jarret (CUSTOM0.w, pivot UV2), pas du cheval en quatre temps,
  galop avec antérieurs repliés, jambe arrière soulagée au repos, blason hors UV = livrée
- [x] `BattleSoldiers` : complet < 32 m, moyen < 75 m, lointain au-delà et pour les ombres
- [x] Capture de gros plan hors simulation : `game/tests/b1_figures_shot.gd`
- [ ] Captures avant/après `docs/img/b1/`, mesures de perf finales, smoke

## Triangles (complet / moyen / lointain ; V4 : complet / allégé)
Fantassin ≈ 4 000 / 950 / 260 (V4 ≈ 440 / 220) ; cavalier ≈ 7 700-9 900 / 1 700-2 050 / 520-690
(V4 ≈ 1 000 / 430).

## Prochaine étape
- Captures avant/après (bataille gros plan et vue d'ensemble, gros plans `b1_figures_shot.gd`).
- Mesures répétées A/B (`--legacy-figures`) : `--benchmark --units=20`, `--benchmark --camera=630,240,28,180`.
