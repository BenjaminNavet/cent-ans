# bm-siege — panneau de siège (Bevy)

État : vue `game_api::siege` (+ 2 tests) et panneau `client/src/campaign/panels/siege/` (mod.rs, view.rs) écrits ; clippy propre.
Prochaine étape : preuve `--dump-state` (CENT_ANS_STAGE_SIEGE + CENT_ANS_OPEN_SIEGE), README, commit final.
Écarts : « Lever le siège » = `SetStance Normal` des armées en posture siège (le siège tombe à la fin du tour ; impossible pour un village, où toute armée hostile assiège). « Attendre » = fermer sans ordre.
Après un assaut, les batailles en attente (`pending_battles`) restent à la charge du flux campagne (équivalent de `_offer_pending_battles`).
