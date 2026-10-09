# RX batsim — état

Branche `rx/batsim` (worktree `../gp-rx-batsim`). ADR 0255.

## Fait (non encore tous mesurés)
- Sonde Rust `core/crates/sim-battle/tests/ai_tactics/rx_batsim.rs` : `rx_survey` (ignoré, 10 et 24 régiments sur pont de pierre / bois + bataille de démo, 8 graines : durée, vainqueur, pertes, déroutes à 0 perte, anéantis sans tué, régiments à l'eau, pic sur le tablier) ; `kills_credited_match_the_enemy_losses` (comptabilité des tués).
- Pont : file d'attente (`crossing_flow_max`, `crossing_queue_reach_m`, `crossing_entry_m` dans `battle_ai.json`), régiments passés tiennent la tête de pont jusqu'à ce que le centre de la ligne ait traversé, marche vers l'entrée du pont plutôt que nage.
- Contagion : `untouched_factor` (battle_rout.json).
- Godot : bandeau de déploiement (erreur sous l'invite, plus de toast au centre), garde `get_date_label` avant `get_character`, `--seed=` et ligne `autoplay:` en fin de bataille (quitte en headless autonome), `smoke_battle` court (`CENT_ANS_SMOKE_ONLY=battle_short`) + étapes horodatées dans le long.

## Reste
- Mesure finale, tests complets sim-battle, ADR 0255, ligne lots.md.
