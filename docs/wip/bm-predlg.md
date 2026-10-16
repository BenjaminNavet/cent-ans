# bm-predlg : dialogue d'avant-bataille (pre_battle_dialog.gd -> flow/dialog.rs)

État : minicarte du site, météo prévue + site, médaillons des généraux portés.
- `SitePreview` (flow/dialog.rs) : `BattleSim` d'aperçu (même setup, site historique et graine que la
  bataille), lecture seule ; trame de `battle::hud::minimap` (relief, obstacles, départs des deux armées).
- Médaillon : portrait `portraits/<id>.png`, sinon écu de la maison (`heraldry/houses/`), sinon blason de faction.
- Ligne « Météo prévue : … · Site : … » sous les conditions ; section de vidage `flow.dialog.preview`.
- Preuve : `client/src/flow/proof_predlg.txt` (CENT_ANS_STAGE_BATTLE=1).
Écarts : pas de bulle d'aide sur les étoiles (tooltip `leader_command_level`) ; ligne météo séparée de `conditions`
(le brief du core donne l'heure, pas la météo). Aucune modification de core/.

small2 : infobulle sur les étoiles de commandement (`flow/dialog.rs::command_tip`, titre `plain.leader_command_level`, « niveau / 10 »). Test `command_tip_tests`.
