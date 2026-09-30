# GA3-L3a — Figurine de bataille générée : longbowman (archer_0)

Branche `feat/ga3`, worktree `../game_project-ga3`. Budget L3a ≤ 2 $ (ligne `docs/budget.md`).
Brutes : `~/dev/cent-ans-raw/ga3/l3/`. Contexte : `docs/wip/ga3.md` (S2, S3), ADR 0140.

## Consigne de reprise
> Lis ce fichier et `git log --oneline -10`, continue à la première case non cochée.

## Choix (décidés, à ne pas rediscuter)
- Référence : `nano-banana-2/edit` (2K) depuis `sr3/longbowman.png` : A-pose, mains vides, 3 vues.
  Zones teintables marquées par des **couleurs clés** dans l'image : jaque = vert saturé
  (livrée), chausses = bleu saturé (étoffe à teinte variable). Segmentation par teinte sur
  l'albédo TRELLIS → masque en alpha de la texture (1 livrée, 0,5 étoffe, 0 fixe).
- 3D : `fal-ai/trellis-2` 1024 sur la vue de face détourée (bria).
- Rig : chaîne S2 (`ga3_figure_probe`) généralisée dans `ga3_figures.py` ; arc, corde, flèche =
  `battle_fine_weapons.longbow` (os `Wrist.L`, `Nock`, `Arrow`) comme `archer_0`.
- Jeu : même format `CAM1` et même chargement (`BattleSkinned`) ; manifeste
  `game/assets/models/battle_ga3/manifest.json` fusionné par-dessus les figurines fines ;
  variante de shader `GA3_TEX` (albédo 1024² sur l'UV du maillage ; codes de matière par face :
  livrée = `C_LIVERY` → livrée/blason/croix du jeu × détail de luminance de la texture).
  `--no-ga3-fig` après `--` = figurine fine actuelle.

## Étapes
- [ ] 1. Squelette (script fal, script Blender, test désactivé, note) — commit
- [ ] 2. Référence NB2 + trellis-2 (≤ 3 essais)
- [ ] 3. `ga3_figures.py` : nettoyage, rig, LOD0/1/2, masque, export CAM1 + albédo + manifeste
- [ ] 4. Jeu : fusion du manifeste, variante `GA3_TEX`, `--no-ga3-fig`
- [ ] 5. Tests (`ga3_l3_figures_test.gd`, smoke, fg3/sr), planche `docs/img/ga3/l3a_archer.jpg`
- [ ] 6. Docs : `ga3.md` § L3a, ADR 0140 § figurines, budget ; verdict extension

## Journal
