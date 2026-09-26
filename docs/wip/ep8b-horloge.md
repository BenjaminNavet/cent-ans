# EP8b — Correctif du bandeau d'heure de bataille

Branche : worktree courant. ADR : 0055 (addendum « EP8b — correctif du bandeau »).

## Bug signalé
Bataille historique d'Azincourt (10 h 30) : le bandeau affiche « Midi » vers 2 min 50 de bataille
jouée. Semble trop rapide.

## Diagnostic
Pas un bug de calcul. `BattleSim::hour` (`core/crates/sim-battle/src/sim/time_of_day.rs`) applique
`minutes_per_battle_second = 0,2` une seule fois, sur `elapsed` (secondes de bataille simulée, pas
à pas fixe `DT = 0,1 s`, incrémenté dans `sim.rs::step`, ligne `self.elapsed += DT`), indépendant
du multiplicateur de vitesse x1/x2/x4 (`battle_scene.gd:816`, `battle.call("tick", delta * speed *
slow)`) : la vitesse ne change que le nombre de pas de 0,1 s exécutés par image, jamais la taille
du pas ni l'unité de `elapsed`. Donc 170 s simulées (2 min 50 à vitesse x1) × 0,2 min/s = 34 min de
jour : 10 h 30 → 11 h 04, dans la phase « midi » (11 h-14 h). C'est exactement la compression
documentée dans l'ADR 0055 (« une bataille de 20 minutes dure 4 heures »), déjà explicite et
volontaire.

Le vrai défaut : lisibilité. `BattleScene._update_time_label` (`game/scripts/battle/
battle_scene.gd`) n'affichait que le nom de la phase (« Midi »), jamais l'heure elle-même. Le
joueur voyait un saut de phase sans repère d'heure, ce qui donnait l'impression d'un bug plutôt
que d'une compression assumée. La fonction de formatage `BattleTimeOfDay.clock_label` (« Midi,
11 h 00 ») existait déjà (écrite pour EP8, testée isolément dans `ep8_staging_test.gd`) mais
n'était jamais branchée sur le bandeau réel.

## Correctif
- `game/scripts/battle/battle_scene.gd` : `_update_time_label` utilise désormais
  `BattleTimeOfDay.clock_label(staging.tod)` et se rafraîchit aussi quand l'heure affichée change
  (pas seulement au changement de phase).
- `data/rules/battle_time_of_day.json` : description enrichie du facteur exact
  (`minutes_per_battle_second`) et de ce que ça donne en minutes de jour par minute jouée.
- `docs/decisions/0055-mise-en-scene-des-batailles.md` : addendum « EP8b — correctif du bandeau ».
- Tests :
  - Rust : `core/crates/sim-battle/src/time_of_day.rs`,
    `azincourt_report_matches_the_documented_compression` — pin le calcul exact (170 s, 10,5 h de
    départ → 11,0667 h, phase « midday »).
  - Godot : `game/tests/ep8b_clock_test.gd` — bataille réelle (`CampaignSim` + `BattleSim`),
    heure de départ forcée à 10,5, 1700 pas de 0,1 s (170 s), vérifie `get_time_of_day().hour` et
    le texte `BattleTimeOfDay.clock_label`.

## État
Fait. Facteur de compression inchangé (choix explicite de l'ADR 0055).

## Vérifications à faire avant de finir
`git merge main`, `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`,
`cargo test --workspace`, `core/build.sh`, `uv run --project tools pytest`, import Godot, smoke.
