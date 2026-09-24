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
- [ ] Squelette (script Blender, chargeur, doc)

## Prochaine étape
- Écrire le script Blender (fantassin, archer, cavalier, cheval).
