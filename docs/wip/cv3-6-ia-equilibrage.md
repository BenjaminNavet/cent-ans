# CV3-6 — IA des postures et des rencontres, équilibrage CV3

Branche : `worktree-agent-abcec86700c5ee43d` (worktree privé, base 5704fa8c).
Spec : `docs/design/2026-09-27-campagne-vivante.md` § 0 et § 5. ADR 0094 (postures), ADR 0085 (bande EQ6).

## Choix
- L'IA de campagne réelle est `core/crates/ai/src/campaign.rs` (`plan_armies`) + `grid.rs` ;
  `sim_campaign::ai_minimal` ne sert qu'aux tests de la simulation : non modifié.
- Nouveau module `core/crates/ai/src/stances.rs` : décisions d'embuscade, de marche forcée, de
  camp retranché et de détour vers une rencontre, appelées depuis `plan_armies`.
- Réglages : sections `postures` et `encounters` de `data/ai/grid.json` (fichier de réglages des
  armées de l'IA ; schéma `ai_grid.schema.json`). Défauts du cœur = rien (comportement antérieur).
- Chaque ordre est vérifié avant d'être donné : `posture::validate_stance_change` et une
  prévisualisation exacte de la marche (`CampaignState::preview_march_to_point`, nouvelle, dans
  `march.rs`, même `simulate` que la vraie marche).
- `GridPlanner` ignore les armées ennemies cachées en embuscade (ni attaquées ni évitées).
- Chances = tirages purs (graine, tour, armée) : planification déterministe, RNG intact.

## État
- [x] Squelette : structures `AiPostures`/`AiEncounters` (data-model), schéma.
- [x] Module `stances.rs` et branchement dans `plan_armies`.
- [x] Tests `ai/tests/cv3_ai_stances.rs` (10) ; tests Python du schéma.
- [x] Sonde : compteurs CV3 (`CV3_STATS=1`) dans `century_probe`.
- [ ] Mesures avant/après (century_probe 464 tours, 4 difficultés, graines EQ6 ; balance_probe).
- [ ] `data/ai/grid.json` : valeurs à écrire APRÈS la fin des sondes « avant » (elles relisent les
  données au lancement ; `bp_before` a un data-model sans les nouveaux champs).

## Mesures exploratoires (century_probe 120 tours, graines 1-2, normale, tout à 1000 ‰)
- Entonnoir de l'embuscade (appels / menace / rapport de force / case couverte) :
  4432 / 86 / 26 / 13 avec les réglages initiaux (80 km, 1 tour, 0,35-0,9) → 0,67 ordre / 20 tours ;
  4181 / 174 / 79 / 19 avec 150 km, 2 tours, 0,25-1,0 → 1,5 ordre / 20 tours (0,75 réussie).
  Le goulot : une armée ennemie en marche de plusieurs tours vers nos terres, et l'embusqué assez
  près de sa route.
- Classe « à la Pyrrhus » : 0 % en auto-résolution même au seuil 0,35 (0,3 % à 0,2) ; les
  vainqueurs perdent rarement 20 % : seuil de la spec (0,5) gardé, la classe reste propre à la 3D.

## Prochaine étape
Attendre la fin des sondes « avant » (`scratchpad/before_*`), écrire `data/ai/grid.json`, lancer
les sondes « après » (`cp_after`, `bp_after`), remplir le tableau, commit final.
