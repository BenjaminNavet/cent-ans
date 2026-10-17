# Rapport UX (parcours joueur) — Cent Ans

Versions : binaire 7e0c4bb12 (runs initiaux) ; le recrutement a été revérifié sur le nouveau binaire (3dd37ab5c), F1+Échap non concluant. Captures et dumps : `$P/ux/` (NN_*.png / .json, scripts s1..s16.txt).
Limites : pas de clic souris sur la carte (script = nœuds UI seulement) ; déplacement d'armée via `CENT_ANS_ARMY_AUTOTEST`. Le sous-titre du conseiller reste affiché en permanence, probablement parce que le jeu est muet (`advisor.speaking` reste vrai) : à vérifier hors mode muet.

## Majeurs
1. **Panneau de recrutement cassé** (7e0c4bb12 ET 3dd37ab5c). Repro : `CENT_ANS_OPEN_RECRUIT=set_paris`, `state Campaign`, shot (ou clic `prov:recruit`). Observé : les cartes d'unités sont écrasées en un liseré de ~8 px (dump : `cards: 10`, `available_units` 5), la liste « Bientôt » (11 lignes) prend toute la place, et le panier / « Sceller la levée / Vider » débordent hors du cadre et chevauchent le panneau province. Après scellage, la file « Annuler » sort du panneau par le haut. Preuves : `21_recruit.png`, `31_sealed.png`, `130_recruit_new.png`. Le recrutement n'est quasi pas utilisable à la souris (le clic par nom marche, mais le joueur ne voit pas les cartes). Cause probable : `client/src/campaign/panels/army/view.rs:842-852` (`scroll_list` à 330 px sans `flex_shrink: 0` ni `min_height`) face à la liste « Bientôt » de `view.rs:888-900`, qui n'est pas dans une zone défilante.
2. **Chargement : l'historique est perdu** (7e0c4bb12). Repro : 3 saisons, sauvegarde, puis Charger (`s5.txt` puis `s6.txt`). L'état `sim` est identique (tour 3, trésor 90346), mais le journal passe de 70 à 0 lignes, la chronique de 28 lettres à 0, `world_lines` de 197 à 0, `feudal.recent` est vidé, et `turns_recorded` passe de 3 à 0. De plus, le tutoriel perd `postponed` (0 devient None) et la réplique d'ouverture du conseiller rejoue. Preuve : `37_saved.json` comparé à `42_loaded.json`.
3. **Raccourcis J (Conseil) et C (Cour) sans effet** (7e0c4bb12). L'aide F1 et la barre du haut affichent J et C, mais `map_toggle_council` et `map_toggle_court` n'ont aucun consommateur dans `client/src` (grep : seulement `keymap.rs`, `help.rs`, `bar.rs`). Le bouton `hud:open_council` fonctionne. Le conseiller dit « Choisissez une technique (T) », or T n'est lié à rien (la touche réelle est F4). Preuves : `100_J.png`, `102_T.png`.
4. **Les raccourcis ouvrent des panneaux pendant la pause** (7e0c4bb12). F3 et F4 pressés pendant que le menu Pause est ouvert ouvrent diplomatie et technologies sous ou avec la pause visible (`64_F3.png`, `65_F4.png`). Le jeu est en pause mais l'écran devient incohérent.
5. **F1 puis Échap** (7e0c4bb12) : l'Échap qui ferme la fenêtre d'aide ouvre aussi le menu Pause (`91_f1_esc.png`, dump `menu.pause`). Cause probable : `menu/pause.rs:101` ne gère que l'aide ouverte depuis le menu. Non reproduit sur 3dd37ab5c, mais le scénario du test était différent (`131_f1esc_new.png`) : à revérifier.
6. **Empilement de fenêtres et de toasts après la fin de saison** (7e0c4bb12). `81_moved.png` : guide féodal (alors que le tutoriel est refusé ou coupé) + offre de mission + panneau d'armée resté ouvert + 4 toasts qui recouvrent le texte de l'offre. `52`, `34_turn3.png` : même chose. Les toasts et sous-titres (z-index supérieur) masquent le titre de la modale « Terminer la saison ? » (`32_enter1.png`) et l'en-tête des panneaux (« ngleterre », onglet « iche » tronqués dans `13_faction.png`). Équivalent Total War : les avis ne recouvrent jamais une décision.
7. **Boîte de dialogue pré-bataille plus haute que l'écran** (1280x800, 7e0c4bb12) : bandeau coupé en haut, boutons Retraite / Résolution automatique / Combattre collés au bord bas (`50_prebattle.png`).

## Mineurs
- Alerte « Aucune recherche en cours : les points de la saison sont perdus » contredite par le panneau Technologies : « 11 points par tour s'accumulent en réserve (0/33) ». Termes incohérents.
- Province Paris : emplacements 9/6 (`slots {max 6, used 9}`, `110_buildings.png`) alors que l'alerte annonce « 32 emplacements libres ». Données ou affichage incohérents. L'onglet Bâtiments n'affiche pas les emplacements sans défilement (pas de barre visible).
- Aide F1 tronquée en bas, sans indice de défilement (`60_f1.png`).
- Rapport de bataille minimal : « Défaite — Général : +10 expérience » (pas de pertes ni de butin ; `aftermath_lines` vide) et titre « Bataille de Île-de-France » (devrait être « d'Île-de-France »). Enter après la fermeture de la modale ne termine pas la saison (une seule action par pression, normal).
- Écran de choix de faction : la carte ne remplit pas son cadre, « Très difficile » passe à la ligne, la carte Angleterre indique « Difficile » alors que le sélecteur affiche « Normale ».
- Briefing de campagne + fenêtre du tutoriel + quote du conseiller s'affichent ensemble au premier écran (`05_tuto.png`) ; le guide féodal arrive dès la saison 2 : surcharge d'onboarding comparée à CK3 ou TW.
- Les 3 premiers tours : un « Terminer la saison ? » avec confirmation à chaque fois tant que des alertes existent (acceptable, mais jamais « ne plus demander »).
- Données : Desmond et Kildare ont le même chef « Maurice FitzGerald ». L'Angleterre démarre en guerre avec 4 puissances.
- Liste de sauvegardes : pas de surbrillance visible de la ligne sélectionnée (`41_loaddlg.png`) ; date au format « 6 h 17 ».
- L'hiver rend des points noirs denses sur la neige (`34_turn3.png`) : à voir côté rendu.

## Ce qui marche bien
- Menu principal complet (Continuer avec sous-titre faction/saison, Charger, Codex, Réglages, Crédits, Quitter) ; invitation au didacticiel claire (Oui/Non).
- Nouvelle partie, choix de faction (fiches, forces/faiblesses), difficulté, fin de saison avec confirmation et liste des alertes, avancement des saisons et trésorerie cohérents (+~21k par saison).
- Sauvegarde : dialogue clair, toast « Partie sauvegardée », autosaves nommées par date, état `sim` restauré à l'identique ; Continuer présent.
- Diplomatie : retour d'information détaillé (« Angleterre refuse (score -72…) » avec raisons et contre-offre), panneaux Objectifs / Réglages / Codex / Armées lisibles, pause complète (Reprendre, Sauvegarder, Charger, Réglages, Aide, Menu principal, Quitter). Aucun ERROR ni panic pendant les runs (hors mes erreurs de nom de nœud).

## Non testé
Déplacement et attaque d'armée à la souris, bataille jouée en 3D (seulement résolue via `CENT_ANS_FLOW_SIM_TO_END`), construction effective d'un bâtiment, accord diplomatique accepté, Quitter (jamais cliqué jusqu'au bout), rechargement sur 3dd37ab5c, Codex règles (`hub:rules`), son et sous-titre hors mode muet.
