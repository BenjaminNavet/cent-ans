# WH diplob — état

Branche `wh/diplob` (worktree `../gp-wh-diplob`), ADR 0282-0283. FAIT, prêt à fusionner.

- Ligue anti-hégémon (`diplomacy/league.rs`), JoinWar, AllyCall, NonAggression, DefensiveAlliance, ultimatums IA : `core/crates/sim-campaign`.
- Données : `data/ai/diplomacy.json` (+ schéma) ; pont : `get_league`, `get_diplomacy` (alliance_kind, non_aggression_turns_left, hegemon), `get_offers` (ultimatum), `treaty_options` (enemies, durées de pacte).
- UI : menus de clauses, boutons d'offres (appel d'allié, ultimatum), fiche de faction.
- Tests : `core/crates/sim-campaign/tests/diplomacy/wh_diplob.rs` (19), `game/tests/wh_diplob_test.gd`.
- Sonde : `campaign_probe` mesure ligue / ultimatums / appels et la domination du premier (voir le rapport du lot).

Restes : l'IA ne propose pas d'alliance défensive (hors ligue) ; JoinWar et pacte de non-agression faits par WR ai-diplo (ADR 0302) ;
la ligue ne se déclenche presque jamais aux seuils actuels ; échecs économie (`eq2_balance`, `b7b_unread_data`) antérieurs au lot.
