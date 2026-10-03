# RJ-b — interpolation des poses de bataille (démarche saccadée)

Retour joueur 03/10 : en bataille, l'animation des figurines est bonne mais leur déplacement
saccade. Chantier parent : `docs/wip/rj-retours-joueur.md`. Worktree `../gp-rj-b`, branche `feat/rj-b`.

## Cause
Pas fixe 0,1 s ; les tampons de figurines (`get_soldier_buffers`) et `get_units` ne changent
qu'à chaque pas : à 60 i/s, figurines figées 5-6 images puis saut. La vitesse estimée par image
(`battle_soldiers.gd`) sur ces positions en escalier fait des dents de scie (cadence de marche
qui pulse à 10 Hz).

## Approche (décision du lot, tient lieu d'ADR)
- Le cœur expose la fraction du pas en cours `step_fraction()` = accumulateur / DT
  (`sim-battle/src/sim/pipeline.rs`, `ReplayPlayer::step_fraction`).
- Le pont garde les poses des deux derniers pas construits (N-k et N) et rend
  `lerp(prev, cur, t)` avec `t = (N - 1 + alpha - (N - k)) / k` (temps affiché = un pas de
  retard, continu même quand plusieurs pas tombent dans une image : vitesse ×N, rattrapage).
  Cap au plus court ; pas d'interpolation quand l'effectif dessiné change, à l'apparition, après
  un changement d'échelle, un saut de plus de `MAX_SPAN` pas (rejeu) ou pour une figurine qui
  saute de plus de `TELEPORT_M` par pas.
- Régiment immobile (poses identiques) : tampon et version inchangés (pas de renvoi au GPU).
- Interpolation côté Rust plutôt que shader : le shader exigerait un second jeu de données
  par instance (données perso. déjà prises par le LOD0 fin, les cadavres, les culbutes) et la
  reprise de tous les traitements GDScript à pas de 12 flottants ; côté Rust, le coût est un
  lerp + sin/cos par figurine mobile et par image.
- Les rangs lâches (PO4, `loosen`) passent en Rust (`set_loose_ranks`) : sinon la boucle
  GDScript par figurine tournerait à chaque image pour chaque régiment mobile proche.
- `get_units` : x, y, z, facing des régiments interpolés de même (dictionnaires du cache mis à
  jour sur place) ; `ground_speed` (m/s, vitesse du centre sur le dernier pas, fournie par le
  pont) remplace la vitesse estimée par image (plus de dents de scie dans `_cadence`/`move_speed`).
- Opt-in `set_pose_lerp(true)` (la scène de bataille, hors `--no-pose-lerp`) : tests et
  headless gardent les poses du pas courant (résultats inchangés).

## État
- [x] Squelette
- [x] Cœur : step_fraction
- [x] Pont : lerp poses + unités + loose (`battle_pose_lerp.rs`, `battle_sim_poses.rs`)
- [x] GDScript : branchement, cadence (skip_far à vérifier)
- [ ] Mesures perf, smoke

## Prochaine étape
Clippy, build GDExtension, `tests/rj_b_pose_lerp_test.gd`, smoke, banc A/B (`--no-pose-lerp`).
