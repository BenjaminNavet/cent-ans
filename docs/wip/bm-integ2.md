# bm-integ2 — câblage inter-lots (migration Bevy)

État : terminé (6 points). bm/uiinfra et bm/econ fusionnés dans la branche.
Fait : fondus FadeToState (menu, faction, pause, batailles, rejeux, flow), ArtPlates (loading, aftermath), bulles filles du Codex sur la ligne du lien, trésor → Économie, historique des batailles jouées (`flow/history.rs`), bandeau illustré + blasons des dialogues terre/mer.
Écarts : historique tenu côté client, non sauvegardé (le cœur n'a pas de champ pour les suites) ; la carte de chargement reste sous le voile du fondu (carte → noir → bataille).
Restes : `campaign/progress/ending.rs` et `battle/scene/mod.rs` (→ menu) en NextState direct (dossiers d'autres lots) ; rechargement depuis la pause sans fondu (Campaign → Campaign) ; pas d'écran listant `PlayedBattles` ; médaillons de général et minicarte du site dans le dialogue.
