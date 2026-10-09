# 0255 — Franchissement d'un pont, contagion de déroute et sonde d'issue (lot RX batsim)

Statut : accepté

## Contexte
La revue d'experts (`docs/wip/rx/bataille.md`, `bataille-v2.md`) a relevé : toute l'armée attaquante s'entasse sur le
pont unique et arrive par paquets (bataille perdue en 5 min, 32 % de pertes contre 14 %) ; des régiments sans perte
(arbalétriers) sont en déroute par contagion ; un régiment « anéanti » affiche 0 tué ; aucune sonde chiffrée de l'issue.
Mesure de départ (`rx_survey`, 24 régiments contre 24 sur pont, 8 graines) : jusqu'à 13 régiments sur le tablier, 6 à
17 déroutés sans perte par bataille, 3 victoires de l'attaquant sur 16.

## Décision
- **File d'attente au pont** (IA, `ai/cover.rs::crossing_queue`, `battle_ai.json`) : au plus `crossing_flow_max` (3)
  régiments de la ligne dans l'eau ou sur le tablier ; ceux qui sont à moins de `crossing_queue_reach_m` (110 m) de
  l'entrée attendent, le plus proche passant d'abord. Les régiments déjà passés tiennent la tête de pont (pas de
  charge au-delà de `counter_charge_distance`) tant que le centre de la ligne n'a pas traversé. Un régiment à plus de
  `crossing_entry_m` (12 m) de l'entrée y va au lieu de traverser à la nage (les noyades en masse venaient de là).
  La cavalerie et les tireurs ne sont pas bridés (voir points ouverts).
- **Contagion** : un régiment qui n'a perdu personne et n'est pas au contact subit la contagion à
  `untouched_factor` (0,35) (`battle_rout.json`, `morale.rs`).
- **Anéanti avec 0 tué** : pas un défaut de comptabilité. Sur terrain sec, les tués crédités égalent les pertes de
  l'adversaire (test `kills_credited_match_the_enemy_losses`, Crécy, ≤ 5 %). Les cas observés sont des régiments de
  fantassins détruits par des flèches ou carreaux avant d'avoir tué quelqu'un, surtout bloqués sur le pont ; le noyé
  n'est crédité à personne. La colonne « Tués » reste donc la vérité.
- **Sonde d'issue** : `cargo test --release -p sim-battle --test ai_tactics -- --ignored --nocapture rx_survey`
  (durée, vainqueur, pertes, déroutes à 0 perte, noyades, pic sur le tablier) ; côté Godot, `--autoplay --seed=<n>`
  écrit une ligne `autoplay: seed=… winner=… end=… t=…s losses=…` et quitte en headless autonome.
- **Smoke** : `CENT_ANS_SMOKE_ONLY=battle_short` (60 s simulées, sans scène) ; `battle` reste le test long, avec une
  ligne `smoke stage …` horodatée par étape.

## Conséquences
- Batailles de pont plus longues (400-600 s au lieu de 300-420 s ; cible EP9 inchangée : 1380 s au plus avec rivière).
- Les fuyards qui traversent la rivière à la nage se noient encore (voulu).
- ADR 0108 (équilibrage siège) n'est pas tranché ici : la sonde porte sur les batailles rangées, pas sur les assauts.
