# bm-bscene2 : rejeu, site historique, navale, autosave (client Bevy, vague 5)

Branche `bm/bscene2`. Suite de `bm-battle.md`, `bm-flow.md`, `bm-bsiege.md`, `bm-bdecor2.md`, `bm-bhud.md`, `bm-orders.md`.

## État (wip)
- Rejeu : `battle/scene/replay.rs` (enregistreur global, fichiers, lecture), `replay_bar.rs` (barre, « Revoir la bataille »), `flow/replays.rs` (menu « Rejeux », message `flow::OpenBattleReplays` à émettre par menu/).
- Site historique : `flow/launch.rs` (`campaign_site`, `CampaignBattle.site`), `HorizonProvince` posé par flow.
- `ResultExtras` : `flow::battles::settle_on_finish` (aftermath) et scène (history).
- Navale : `flow/naval.rs`. Autosave : `flow::battles::autosave_before_battle`. Caméra far 14 000 m.
- Édits hors dossier : `battle/orders/{input,deploy,formations}.rs` (appels via `replay::*` pour enregistrer).

## Prochaine étape
Preuves par script (`CENT_ANS_REPLAY_DEMO`, `CENT_ANS_FLOW_FINISH`, `CENT_ANS_STAGE_NAVAL`), clippy, tests.
