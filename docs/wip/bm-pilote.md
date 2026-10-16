# Partie pilote Bevy (bm-pilote)

Joueur : agent, via la surface de contrôle (`run_bg.sh`, debug + `CENT_ANS_FEATURES=dev`, `--offscreen`, `CENT_ANS_MUTE=1`, dossier de sauvegardes et réglages dans un répertoire temporaire). Tête 2d32d2f16. Aucune capture, preuve par `dump`. Scripts et dumps : `/private/tmp/claude-501/pilot/` (non versionnés).

## Ce qui marche (vérifié par dump)
- Menu principal, nouvelle partie, carte de factions, France 1337 normal, invite du didacticiel (`Non`), briefing (`À la guerre !`), campagne tour 0 (28 000 ₶).
- Recherche (`tech:tech_pavise`, 130 pts, 8 tours), construction (`prov:build:bld_water_mill`, `bld_armoury` : ordre accepté, trésor débité), recrutement (`army:recruit`, `rec:unit:…`, `rec:seal` : file 1, −450 ₶), déplacement d'armée (hooks `CENT_ANS_ARMY_KEYS=1` + Tab/KeyM), diplomatie (proposition de paix à l'Angleterre refusée avec raisons chiffrées, présents à la Bourgogne −1000 ₶, acceptation d'offres `dip:offer:N:accept`, décision de chronique `hud:decision_option:0`).
- Panneaux ouverts et confirmés : province (par `hud:alert:free_slot`), armée, recrutement, diplomatie (F3), tech (F4), conseil, codex (surcouche en campagne), faction, économie, chronique, féodal, traditions, siège.
- 10 tours enchaînés (T0-T4 puis T4-T10) ; sauvegarde (`pause:save`) puis chargement (`Charger une partie`, `slot:partie`) : empreinte d'état identique (851e9c15686d14c8 aux deux sessions, déterminisme OK), recherche et file conservées.
- Bataille de campagne jouée (pas auto-résolue) : site de rencontre (`CENT_ANS_STAGE_ENCOUNTER=1`, option 1) → dialogue d'avant-bataille → discours, déploiement → bataille Île-de-France (8 vs 2 régiments), sélection/déplacement/attaque via `CENT_ANS_ORDERS_DEMO=1`, vitesse ×4, fin à tick ≈ 2100-2500, écran « Victoire », `result:return` → campagne (armée 538 hommes, +20 xp, 8 régiments aguerris, historique des batailles).
- Siège : panneau (rôle, vivres, brèche, engins, garnison), `siege:summon` accepté, `siege:assault` refusé avec le motif exact du cœur (échelles dans 6 tours). Siège de démo de Paris lancé en bataille 3D (24 pièces, bélier, tick 1491, 421→301 défenseurs).
- Pause (Échap, `hud:menu`), réglages (`pause:settings`, onglet audio, curseur +0,1, `settings:cancel` restaure 0,6), retour au menu, `Quitter` : sortie propre, code 0, aucun panic.

## Bogues
| # | Gravité | Description | Repro minimale |
|---|---|---|---|
| 1 | gênant | Échap sur le panneau Tech le ferme ET ouvre le menu pause (les autres panneaux — province, recrutement, armée, diplomatie, chronique, faction, féodal — sont corrects). Le doc de `pause_escape` demande de mettre `KeyEdges::escape` à faux. | nouvelle partie France, `click À la guerre !`, `click hud:open_tech`, `key Escape`, dump : `tech.open=false` et `menu.paused=true`, `menu.overlays=["PauseMenu@0"]` |
| 2 | gênant | `hud:open_recruitment` sans armée sélectionnée n'ouvre rien (l'infobulle promet la capitale) : `OpenPanel::Recruitment("")` via `unwrap_or_default()` dans `campaign/hud/mod.rs` ~l.784. Avec armée sélectionnée, ça marche. | campagne T0 ou chargée, `click hud:open_recruitment`, dump : `ui_kit.panel_stack.visible=[]` |
| 3 | mineur | Journal : ≈ 7 000 `WARN bevy_render::mesh::allocator: Mesh … contains no vertices` par bataille (25 099 sur la partie). Un maillage de bataille est vide. | n'importe quelle bataille (`CENT_ANS_STAGE_ENCOUNTER=1` option 1 puis `flow:fight`), `grep -c "contains no vertices"` |
| 4 | mineur | Après `result:return`, `ui_kit.modal.active="flow:battle_result"` reste actif en campagne et `Confirmer` (fin de tour) n'existe pas : je n'ai pas trouvé le bouton qui ferme cette modale par script (non vérifié si c'est un artefact du dump pris 300 images après). | script `sK` : bataille jusqu'au bout, `result:return`, wait 300, dump `ui_kit.modal.active` |
| 5 | mineur | Dialogue de sauvegarde : si le nom existe déjà, le premier `dialog:confirm` ne fait rien visible dans le dump (dialogue ouvert, `saved_turn` inchangé), le second écrase. Pas d'état « Écraser ? » exposé. | sauvegarder deux fois sous `partie` dans une session |
| 6 | mineur (dev) | `CENT_ANS_STAGE_BATTLE=1` ne fait rien et sans journal quand `debug_stage_battle` échoue (aucun log « staging failed ») : au tour 4, une armée hostile existait mais aucune bataille n'a été montée. | charger la partie du T4, `CENT_ANS_STAGE_BATTLE=1`, attendre : `flow.dialog=null` |
| 7 | observation | La durée de la bataille dépend du rythme des images (même script : fin à tick 2118, 2498, ou non finie à 3021). À confirmer si voulu (pas de pas fixe de simulation de bataille). | rejouer `sK`/`sJ` |
| 8 | observation | Siège monté par `CENT_ANS_STAGE_SIEGE=prov_guyenne` : après 1 tour le panneau passe à `role=ended`, sans motif visible ; manpower tombe à 498 et revenu net 17,6 k → 8,7 k au T10 sans cause identifiée. | `CENT_ANS_STAGE_SIEGE=prov_guyenne`, 6 × (`click hud:end_turn`, `click Confirmer`) |

## Limites de la surface de contrôle (pas des bogues du jeu)
- Pas de pointeur dans les scripts : aucune sélection, déplacement ou charge à la souris. Les ordres de bataille ne passent que par `CENT_ANS_ORDERS_DEMO=1` (clic, boîte, clic droit, glisser, attaque). `Command_pursue`, `Mode_run`, `speed:4`, `order:*`, `unitcard:N` sont acceptés (`commands.sent`) mais `Command_pursue` laisse les unités `idle` quand l'ennemi est loin ; la minicarte (`click Minimap`) n'envoie aucune destination.
- Le clic sur un nom inexistant attend 300 images puis est fatal (`AGENT_FATAL`) : un script ne peut pas être conditionnel (ex. `Confirmer` n'apparaît que s'il y a une alerte bloquante).
- La fenêtre de décision (`hud:decision_window`) et la modale de fin de tour ne sont pas dans le dump (on lit `ui_kit.modal.active`).

## Non exercé et pourquoi
- Bataille née d'un contact réel sur la carte (déplacer l'armée sur une armée ennemie, p. ex. Saintonge T4) : pas de clic carte ; `STAGE_BATTLE` muet (bogue 6) → remplacé par un site de rencontre.
- Assaut de siège de campagne jusqu'à la bataille : engins jamais prêts avant la fin du siège (bogue 8).
- Placement du déploiement à la souris, formations, bataille navale, `Continuer`, F1/F2/F5-F12, ponts de changement de faction, difficultés autres que normal.
- Aucune capture : aucun jugement visuel (rendu, lisibilité) n'a été fait.
