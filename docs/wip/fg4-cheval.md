# FG4 — Cheval fin en production

Branche : `feat/fg4-horse` (worktree agent). Plan : `docs/wip/fg-figurines-fines.md`.
Prototype : `docs/wip/fg0-prototype.md`.

## État
- [ ] Squelette : `tools/blender_scripts/battle_fine_cavalry.py` (pipeline de production)
- [ ] Déformation : poids des jambes (jarrets, boulets, paturons, épaules, grasset), tête, yeux
- [ ] Étriers +6 cm dans `battle_skinned_cavalry.py`, `cavalry.bones.bin` recuit
- [ ] Harnachement : selle, bride, rênes, chanfrein, caparaçon deux pièces, flançois
- [ ] Types de chevaux (destrier / roncin / genet) et robes (teintes du shader)
- [ ] Faces cachées sous le caparaçon supprimées
- [ ] Chaîne LOD (≈ 5-6 k / 1 000-1 200 / ≤ 300) exportée en `CAM1` dans `battle_fine/`
- [ ] Planches `docs/img/fg/fg4_*.png`

## Prochaine étape
Squelette du script, puis reprise des poids.
