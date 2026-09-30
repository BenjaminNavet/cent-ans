# IA — nuit du 2026-09-30 : IA performante en campagne et en bataille

Branche `feat/ia` (worktree `../gp-ia`, depuis `main` 6a0d960c9). Mandat du joueur : « tu
travailles sur l'IA du jeu et tu t'assures qu'elle soit performante en campagne ou en bataille »,
autonomie toute la nuit.

« Performante » = les deux sens : l'IA joue bien (bataille : bat une IA naïve à forces égales,
exploite terrain et armes ; campagne : défend, prend les villes vides, ne se ruine pas) et elle
est rapide (tour de campagne < 50 ms par faction, spec M3 ; coût de l'IA par tick de bataille).

## Outils ajoutés
- `core/crates/sim-battle/tests/ia_survey.rs` (ignoré) : `survey` = IA contre un novice (chaque
  régiment attaque l'ennemi le plus proche toutes les 2 s), contre un camp passif, contre elle-même ;
  armées miroir / anglaise / française ; 4 terrains ; `IA_SELF=naive` = le camp mesuré joué en
  novice (référence). `plan_cost` = coût de `ai::plan` seul.
  `cargo test --release -p sim-battle --test ia_survey -- --ignored --nocapture survey`
  (16 graines ≈ 6 min).
- `core/crates/ai/examples/turn_hotspot.rs <seed> <tour> <faction> [reps]` : rejoue jusqu'au
  tour, chronomètre la planification d'une faction (`HOTSPOT_WAIT=1` pour `sample <pid>`).
- `core/crates/ai/examples/ia_quality_probe.rs` (agent) : qualité de l'IA de campagne.

## Mesures de départ (main 6a0d960c9)
- Coût bataille : `ai::plan` des deux camps 0,03 / 0,23 / 0,48 ms à 20 / 63 / 105 régiments par
  camp, une fois toutes les 2 s : négligeable.
- Coût campagne (`turn_perf --sequential 10 1 1`, machine chargée) : moyenne 5,1 ms, p99 22 ms,
  **pire 548 ms (Venise/Vérone t3)** → premier `ambush_orders` décode la carte de couverture
  (`cover_class_at`, OnceLock, ~1,2 s). Corrigé : préchauffage des rasters dans un fil au
  chargement des données (`godot-bridge/src/campaign_sim.rs load_shared_data`).
- Bataille, 16 graines × 4 terrains × 2 camps (victoires IA / 64) :

| adversaire | armées | IA attaque | IA défend |
|---|---|---|---|
| novice | miroir | 61 | 52 |
| novice | IA anglaise contre fr. | 61 | 63 |
| novice | IA française contre angl. | 38 | 48 |
| passif | miroir | 50 | 61 |
| passif | IA française contre angl. | 5 | 51 |
| IA | miroir | 30 | 34 |
| IA | IA française contre angl. | 2 | 19 |

  Référence novice contre novice : française contre anglaise 2/64 → l'IA apporte beaucoup.
  L'armée anglaise (5 000) bat la française (6 100) presque toujours : équilibre des unités
  (arc long), pas l'IA.
- Défaut vu (trace `IA_LOG=1`) : les chevaliers français attendent l'infanterie (EQ7) à leur poste
  d'aile, calé sur l'ancre d'avance de la ligne (30 m devant l'infanterie), à portée des arcs
  longs : 60 % de pertes sans frapper, puis charge épuisée et déroute.

## Essais
- w1 : poste d'aile jamais devant l'infanterie + recul hors de portée des tireurs tant que
  l'infanterie n'est pas en mêlée. Novice fr. attaque 38 → 51 ; mais passif miroir attaque 50 → 30,
  passif fr. défend 51 → 31 (la cavalerie arrive trop tard). Variantes en cours (`IA_WING`).

## Prochaine étape
Choisir la variante du poste d'aile ; puis sonde de campagne (agent) → correctifs campagne.
