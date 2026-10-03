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

- Budget EP1 (`_skip_far`) : décidé avant l'appel ; un régiment sauté reçoit sa capacité codée
  `-2 - capacité` et le cœur lui rend son tampon tel quel (pas de lerp inutile). Gardé : au-delà
  de 450 m, une mise à jour toutes les 2-3 images d'une position déjà interpolée bouge de
  quelques centimètres (< 1 px), plus de saut de pas entier.
- `battle_effects.gd` : le cache d'emprise mouillée était indexé sur la position exacte
  (recalculé à chaque image avec l'interpolation : +0,9 ms) ; tolérance 0,5 m / 0,05 rad.

## Mesures (bibliothèque debug, `tools/bench_ep1.sh`, vsync à 60 i/s, machine partagée)
| Banc | `proc_plain` médiane | figurines (images sans pas) | `proc_step` médiane |
|---|---|---|---|
| 80 régiments, 9 598 fig., `--units=40 --autoplay --bench-at=40`, lerp | 5,38 ms | 2,00 ms | 7,07 ms |
| idem `--no-pose-lerp` | 4,97 ms | 1,76 ms | 7,27 ms |
| `--closeup --autoplay`, lerp | 1,67 ms | 0,65 ms | 2,05 ms |
| idem `--no-pose-lerp` | 1,58 ms | 0,59 ms | 2,13 ms |

Test headless (`tests/rj_b_pose_lerp_test.gd`) : figurine de tête qui bouge sur 114/119 images
avec interpolation contre 20/119 sans ; 14 régiments immobiles gardent leur version de tampon.

## État
- [x] Squelette, cœur (`step_fraction`), pont, GDScript, banc A/B
- [x] smoke.gd vert (normal et `-- --pose-lerp`). Note : dans ce worktree, le chargement de la
  carte de campagne en headless crache des millions d'erreurs `geometry_instance is null`
  (`river_labels.gd:118`, Label3D) : sans lien avec le lot, filtrer la sortie.

## Points ouverts
- Écart à la spec : quand l'effectif dessiné change (pertes), les rangs communs restent
  interpolés (garde-fou de téléportation) au lieu d'un retour à la pose courante : sinon chaque
  perte d'un régiment en marche sous les traits refait un saut.
- `_previous` (figure_at, cadavres, étendards) tient désormais les places « lâchées » et
  interpolées (rangs lâches appliqués par le cœur à toute distance, ±0,15 m, ±4°).
- Coût restant : ~+0,4 ms par image sans pas sur 9 600 figurines en bibliothèque debug
  (lerp Rust non optimisé + envoi des tampons des régiments en marche).
- Vérification visuelle en jeu par le joueur (pas de capture dans ce lot).

## Prochaine étape
Relecture et fusion par l'orchestrateur (`../gp-rj-merge`).
