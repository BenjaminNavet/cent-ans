# bm-bscene2 : rejeu, site historique, navale, autosave (client Bevy, vague 5)

Branche `bm/bscene2`. Suite de `bm-battle.md`, `bm-flow.md`, `bm-bsiege.md`, `bm-bdecor2.md`, `bm-bhud.md`, `bm-orders.md`. Détails et noms de scripts : `client/README.md` § « Bataille : rejeu, site historique, batailles navales ».

## Fait (Godot -> Rust)
- `battle_replay_controller.gd`, `godot-bridge/battle_replay.rs` -> `battle/scene/replay.rs` (enregistreur, fichiers `rejeu-*.json`, lecture, sauts, refus des ordres) ; `battle_replay_bar.gd` -> `replay_bar.rs` ; `replays_menu.gd` -> `flow/replays.rs` ; « Revoir la bataille » (bouton sous « Retour à la campagne »).
- EP7 : `flow/launch.rs` (`campaign_site`, `CampaignBattle.site`, `start_of`) ; `HorizonProvince` posé par flow.
- `ResultExtras` : `flow::battles::settle_on_finish` (aftermath), scène (history).
- `naval/naval_pre_battle_dialog.gd` -> `flow/naval.rs`. `autosave_before_battle` -> `flow::battles`. Caméra far 14 000 m.
- Preuves : dump `battle.replay.demo.matches = true` (empreinte de l'état identique, bataille de 1 507 pas), saut en arrière, quitter ; campagne -> bataille -> fin -> Revoir -> Quitter ; naval ; menu Rejeux.

## Édits hors dossier
- `battle/orders/{input,deploy,formations}.rs` : appels via `replay::*` (enregistrement + refus pendant un rejeu).
- `battle/hud/result.rs` : un `Name` en double faisait paniquer l'écran de fin (jamais ouvert en vrai avant) ; retiré.
- `campaign/armies/sea.rs` : `auto_resolve_pending` s'efface devant le dialogue naval.

## Restes
- Le menu principal doit émettre `flow::OpenBattleReplays` (« Rejeux » reste grisé dans menu/) ; en attendant `CENT_ANS_OPEN_REPLAYS=1`.
- Écarts : date des rejeux en UTC (Godot : heure locale) ; un saut en arrière recharge l'écran de bataille (caméra gardée) ; touches +/− de vitesse non portées ; le HUD garde ses boutons d'ordres pendant un rejeu (refusés) ; pas de minicarte/bannière illustrée/météo prévue dans les dialogues (terrestre et naval) ; flottes sans héraldique.
- Enregistrement : les accroches de dev qui touchent la sim sans `replay::*` (`CENT_ANS_UNITS_SPEED=…,1`, `CENT_ANS_SIEGE_*`) ne sont pas enregistrées (divergence au rejeu).
- Bataille de campagne sur site : seul le test unitaire le prouve (aucune bataille mise en scène ne tombe sur une province de carte historique).
