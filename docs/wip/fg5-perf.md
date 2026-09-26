# FG5 — Performance et bascule des figurines fines par défaut

Branche : `feat/fg5-perf` (worktree `.claude/worktrees/fg5-perf`). Plan : `docs/wip/fg-figurines-fines.md`.
Précédents : `fg1-corps.md`, `fg3-matieres.md`, ADR 0088. Décision : **ADR 0089**.

## État : TERMINÉ 26/09 (branche prête, non fusionnée dans main)
- [x] Bancs de référence (standard et rapproché, 4 passes alternées)
- [x] LOD0 par soldat (calque compacté, frustum, `lod_band` / `id_in_custom` en `instance uniform`)
- [x] LOD1 et LOD2 allégés, recuisson complète (`battle_fine.py -- bake`, ~45 min)
- [x] Bascule : rendu fin par défaut ; `--coarse-figures` (Quaternius), `--no-fg3`, `--fine-figures`
  accepté sans effet ; `--legacy-figures` inchangé (figurines rigides)
- [x] Vérifs : smoke, `bv3_check`, `pb3c_buffers_test`, `pf1_quality_test`, `ep13_replay_test`,
  `fg3_maps_test` (défaut et `--coarse-figures`), `ep12_shot` (contrôle) : OK
- [x] Captures `docs/img/fg/fg5_*.png` : mêlée 26 m et 9 m (fine | coarse), campagne, menu,
  cadavres, imposteurs, étendards (monté, ligne), armoiries DA1, blessés EP12
- [x] ADR 0089, bible § 6, journal FG

## Journal
- Banc standard (`--units=50 --bench-at=90`, 1600×900, `--quality=ultra`) : la caméra ne voit
  **aucun** régiment en LOD0/LOD1 (25 en LOD2, 75 imposteurs) : l'écart fin/défaut (+8 %) vient
  du LOD2 (335/600 triangles contre 230/340) et de la variante `FG3_BAKED`. Médiane collée aux
  paliers d'affichage (10 ms à 100 Hz en « high », 16,7 ms) : métrique retenue = durée moyenne
  d'image (1000 / i/s), la médiane est notée à part.
- Banc rapproché ajouté (`--benchmark --closeup` : caméra à 26 m du régiment le plus proche de
  l'ennemi) : fine = **+94 %** (29,9 ms contre 15,4), primitives 5,1 → 10,6 M. Sans LOD0 : +15 %,
  sans LOD0 ni LOD1 : -3 %. Cause : le LOD choisi par régiment (centre, lignes de 40-150 m) :
  tout un régiment passe en LOD0 (9-17 k triangles × 120).
- 1er essai : calque LOD0 avec tout le tampon, repli dans le shader hors bande : encore +70 %
  (709 soldats × 10 k sommets). 2e : tampon compacté (rang en donnée perso) : +46 % (125
  soldats). 3e : + test du champ de la caméra : **+19 %** (82 soldats en LOD0).
- `fine_distance` 40 m, tuiles seulement au LOD0 : aucun effet mesurable (gardés tels quels).

## Banc final (Ultra, 4 passes alternées, durée moyenne d'image)
| | Quaternius (`--coarse-figures`) | fine (défaut) | fine `--no-fg3` |
|---|---|---|---|
| standard | 16,58 ms | 17,04 ms (+2,8 %) | 16,76 ms (+1,1 %) |
| rapproché (`--closeup`) | 15,15 ms | 18,09 ms (+19 %) | — |
Élevée : standard 10,12 / 10,68 ms (+5,4 %, une passe bruitée ; +1,3 % sans elle), rapproché
10,26 / 11,91 ms (+16 %).

## Points ouverts
- Vue rapprochée : +16 à +19 %, le prix des figurines fines réellement vues (LOD0 < 12 m,
  LOD1 fin 2,5× plus lourd que Quaternius). Piste : LOD0 plus léger au-delà de ~8 m (LOD « 0,5 »).
- Soldats renversés (`_tumble_layers`), duels, porte-étendards détachés : toujours au LOD0
  (peu nombreux).
- Retirer `--coarse-figures` et `battle_skinned/` après la transition.
- La capture `--standard-shot=foot` cadre une pile de pont (hors FG5, cadrage de la scène).

## Commandes
```
cp /Users/jean_hubert/dev/game_project/game/bin/*.dylib game/bin/
godot --headless --path game --import
godot --path game --resolution 1600x900 res://scenes/battle/battle.tscn -- --units=50 --benchmark --bench-at=90 --quality=ultra [--closeup] [--coarse-figures]
godot --path game --resolution 1600x900 res://scenes/battle/battle.tscn -- --closeup --closeup-distance=9 --no-hud --screenshot=<png>
godot --path game --resolution 1600x900 --script res://tests/fg5_campaign_shot.gd -- --out=<png> --distance=5 --camera-min=2
blender -b --factory-startup --python tools/blender_scripts/battle_fine.py -- bake
```
Pièges : `--path game` (pas la racine du worktree : « [unnamed project] » qui attend sans fin) ;
zsh ne découpe pas les variables (`$G` de plusieurs mots) ; les bancs fenêtrés demandent de
sortir du bac à sable.
