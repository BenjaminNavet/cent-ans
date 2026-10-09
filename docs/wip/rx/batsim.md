# RX batsim — état

Branche `rx/batsim` (worktree `../gp-rx-batsim`). ADR 0255.

## Fait (non encore tous mesurés)
- Sonde Rust `core/crates/sim-battle/tests/ai_tactics/rx_batsim.rs` : `rx_survey` (ignoré, 10 et 24 régiments sur pont de pierre / bois + bataille de démo, 8 graines : durée, vainqueur, pertes, déroutes à 0 perte, anéantis sans tué, régiments à l'eau, pic sur le tablier) ; `kills_credited_match_the_enemy_losses` (comptabilité des tués).
- Pont : file d'attente (`crossing_flow_max`, `crossing_queue_reach_m`, `crossing_entry_m` dans `battle_ai.json`), régiments passés tiennent la tête de pont jusqu'à ce que le centre de la ligne ait traversé, marche vers l'entrée du pont plutôt que nage.
- Contagion : `untouched_factor` (battle_rout.json).
- Godot : bandeau de déploiement (erreur sous l'invite, plus de toast au centre), garde `get_date_label` avant `get_character`, `--seed=` et ligne `autoplay:` en fin de bataille (quitte en headless autonome), `smoke_battle` court (`CENT_ANS_SMOKE_ONLY=battle_short`) + étapes horodatées dans le long.

## Mesures (24 contre 24 sur pont, 8 graines)
- Avant : pic 7-13 régiments sur le tablier, 6-17 déroutés sans perte, attaquant gagnant 3/16, 3-12 noyades.
- Après : pic 4-8, 0-12 déroutés sans perte (médiane ~6), attaquant gagnant 9/16, durées 380-620 s (2 à ~1020 s).
- Tués crédités = pertes adverses à 5 % sur terrain sec (Crécy) ; l'écart sur pont = noyades (non créditées).
- Contagion : l'effet de `untouched_factor` reste faible (56 -> 50 déroutés sans perte sur 192) ; la cause principale des déroutes à 0 perte n'est pas la contagion (fantassins/archers longs). Restreint aux tireurs : appliqué à tous les régiments il faisait passer `symmetric_flat_battle_is_open` à 1/10.

## Reste
- Cavalerie et tireurs non bridés au pont ; cause des déroutes à 0 perte à creuser (fatigue, flanc, moral de base).
- Godot non lancé (smoke court, `--autoplay --seed`) : la dylib doit être reconstruite (`core/build.sh`).
- ADR 0108 non tranché (siège).
