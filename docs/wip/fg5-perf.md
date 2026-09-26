# FG5 — Performance et bascule des figurines fines par défaut

Branche : `feat/fg5-perf` (worktree `.claude/worktrees/fg5-perf`). Plan : `docs/wip/fg-figurines-fines.md`.
Précédents : `fg1-corps.md`, `fg3-matieres.md`, ADR 0088.

## État
- [ ] Banc de référence (défaut / fine, 4 passes alternées)
- [ ] Leviers de perf : relais LOD0 plus tôt, `fine_distance` 80 → 40 m, tuiles au LOD0 seulement,
  LOD1 allégé (décimation + recuisson)
- [ ] Bascule : rendu fin par défaut, drapeau pour l'ancien rendu skinné, `--no-fg3` gardé
- [ ] Vérifs : smoke, tests figurines, captures `docs/img/fg/fg5_*.png` (campagne, menu, cadavres,
  imposteurs, étendards, DA1, EP12)
- [ ] ADR 0089 + bible § 6 + journal FG

## Prochaine étape
Banc de référence.

## Commandes
```
cp /Users/jean_hubert/dev/game_project/game/bin/*.dylib game/bin/
godot --headless --path game --import
godot --path game --resolution 1600x900 res://scenes/battle/battle.tscn -- --units=50 --benchmark --bench-at=90 [--fine-figures]
```
Piège : `--path game` (pas la racine du worktree : « [unnamed project] » qui attend sans fin).

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
- Réponse : LOD0 **par soldat** pour les figurines fines (calque LOD0 à part, même tampon, bande
  de distance `lod_band` en `instance uniform` : hors bande, soldat replié avant skinning),
  rayon `FINE_DETAIL_DISTANCE` 12 m × préréglage ; LOD1 1 350 (à pied) / 1 000 (cavalier) + cheval
  ~700, LOD2 260 / 180 + cheval ~190 (recuisson complète `battle_fine.py -- bake`).
