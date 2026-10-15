# BM mapui : fenêtre de sort d'une place prise

## État
- `client/src/campaign/hud/capture.rs` (port de `capture_controller.gd`) : fenêtre modale auto-ouverte pour chaque `PendingCapture` du joueur, 4 choix chiffrés (`capture_decision_views`), raser grisé avec sa raison, « Plus tard » (non rouverte), ordre `Order::ChooseCaptureOutcome`. Dump `hud_capture`. Hook dev `CENT_ANS_DEBUG_CAPTURE=<id|auto>`.

## Déjà porté ailleurs (rien à refaire)
- `mission_offer_controller` : `hud/missions.rs` (offre + `ChooseMission`).
- `help_controller` / F1 : `hud/help.rs`. `council_controller` : bouton `hud:open_council` + `panels/court`.
- `attack_cursor` / `campaign_cursor` : `armies/feedback.rs`.

## Non portés (inutiles)
- `army_memory`, `heraldry_atlas` : aucune référence dans Godot (code mort). `map_paths` (dossier data : `CENT_ANS_ROOT`), `province_snapshot` (remplacé par game-api), `panel_widgets` (remplacé par `ui/kit`).
