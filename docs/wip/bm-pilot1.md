# bm-pilot1 : correctifs issus de la partie pilote

Preuves : dumps du bloc-notes `/private/tmp/claude-501/pilot1/` (scripts camp/sg/sv/crecy/siege.txt, `run.sh`).

1. Échap sur Tech (F4) : `open_close` consomme `KeyEdges::escape` et passe `.before(pause_escape)`. Dump : `tech.open` vrai puis faux, `menu.paused` faux.
2. `hud:open_recruitment` sans armée choisie ouvre la capitale du joueur (`ui_kit.panel_stack.visible=["recruitment"]`).
3. « Mesh contains no vertices » : source = `ArrowTrails` (`battle/effects/trail.rs`), rubans vides réécrits chaque image. Maintenant triangle dégénéré de remplacement et aucune réécriture tant qu'il n'y a pas de flèche. 0 avertissement sur Crécy (900 images), siège de Paris, bataille de campagne (600 images). Sonde ajoutée : `CENT_ANS_PROBE_EMPTY_MESH=1` (dev) journalise nom, parents et composants de toute entité qui dessine un maillage vide à son apparition (les maillages extraits ne sont plus lisibles côté jeu). Aucune autre source trouvée.
4. Sauvegarde, nom existant : message « existe déjà. Écraser cette sauvegarde ? », bouton `dialog:confirm` renommé « Écraser », dump `menu.saves.dialog.overwrite_armed`.
5. `CENT_ANS_STAGE_BATTLE` : le montage fonctionne (journal `staged battle`) quand une armée du joueur est dans une place ; sinon warn `no battle could be staged (N armées, M en place, K hostiles)`. Cause du silence du pilote : le cœur (`debug_stage_battle`) exige l'attaquant dans un établissement, une armée en marche échoue.
6. `flow:battle_result` : c'est la notice voulue du retour de bataille (bouton « Fermer », pas « Confirmer »). Gardée (elle porte les pertes), mais le dump expose `ui_kit.modal.active_buttons` et `click_if Fermer` la ferme. Bogue réel trouvé en passant : le briefing de tour 0 se réaffichait au retour d'une bataille jouée au tour 0 ; corrigé (`briefing.rs`, compteur de batailles quittées).
7. Siège `STAGE_SIEGE=prov_guyenne` : cœur, pas client. Le montage fabrique un siège (4 tours, vivres 55) ; au tour suivant l'armée assiégeante passe de 620 à 321 hommes, un rapport de bataille et un « Traité de paix » apparaissent : la bataille / la paix levée par le cœur ferment le siège (`role=ended`). Les revenus nets de ~19 k et la baisse d'effectifs viennent de la partie, pas d'un défaut d'affichage. Le panneau ne dit pas pourquoi le siège a fini : piste (non faite) = lire l'événement dans le journal du tour.
8. Script : `click_if <nom>` (alias `try_click`), non fatal, saute après 10 images ; README client.

Restes : motif de fin de siège dans le panneau ; réinitialiser la légende « Écraser » si le nom change après l'armement.
