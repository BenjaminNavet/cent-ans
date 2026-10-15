# bm-bhud : HUD de bataille (client Bevy)

État : lots bhud + bhud2 faits (cartes, alertes, vitesse, comparaison, résultat, infobulles ; minicarte, sceau du chef, barre des ordres, capacités sur les cartes, journal, aide F1, état de siège, cartes de suites, carte historique, fond peint). Tests unitaires par module, vérifié en `--script` Battle (section `battle_hud`).

## bhud2 (Godot -> Rust, client/src/battle/hud/)
- battle_minimap.gd -> minimap.rs (image CPU 2x, clic/glisser = caméra, clic droit = `MoveSelectionTo`, repère pulsé `MinimapPing` envoyé par la colonne d'alertes). Placée à gauche de la colonne de vitesse (`BAND_RIGHT_MARGIN` recalculée : 8 + 72 + 6 + 168 + 6).
- leader_orders_bar.gd + sceau de battle_hud.gd -> orders_bar.rs (ordres lus par `leader_orders` sérialisé, touches Z X V B / I, `IssueBattleCommand(LeaderOrder|Burn)`). Sceau : disque de cire, portrait, anneau de moral (couleur de bordure), clic = sélection, double clic = caméra. Cartes décalées de `BAND_LEFT_MARGIN`.
- battle_ability_icons.gd (affichage) -> abilities.rs : 4 boutons sous chaque carte, clic = message `AbilityRequested {unit, ability, slot}` (rien d'autre : le lot bdeploy l'applique par `IssueBattleCommand(UseAbility)`, Alt+1..4 aussi).
- journal (battle_hud.gd) -> journal.rs (J, 8 lignes, regroupement x2 sous 8 s).
- aide F1 + état de siège -> info.rs.
- cartes de suites / carte « Dans l'Histoire » / fond peint (battle_result_screen.gd, ArtPlates.ending) -> aftermath.rs + result.rs. `ResultExtras { aftermath, history }` est à remplir par flow / la scène : l'`Aftermath` de flow n'existe qu'après `LeaveBattle`, et la scène ne sait pas lancer une bataille historique.

## Restes
- Menu des formations (battle_formation_menu.gd), « Revoir la bataille », ligne « Effets » (RuleValues) de l'infobulle des capacités, daltonisme de la minicarte (losanges toujours actifs), libellés de touches selon la disposition AZERTY, repli de l'ordre de chef sur tous les régiments quand la sélection ne peut pas obéir (l'issue de la commande n'est pas lue).
- `ResultExtras` à brancher (flow : appliquer l'issue avant d'ouvrir l'écran ; menu historique : `HistoryInfo`).

## bhud3 (vague 5)
Faits : menu des formations (`formation_menu.rs`, bouton « Formation » au-dessus du sceau, ordre `Command::Formation` par `IssueBattleCommand` pour les régiments sélectionnés qui peuvent la prendre), ligne « Effets » de l'infobulle des capacités (catalogue du cœur), libellés de touches des ordres du chef selon la disposition (AZERTY : W X V B, aide F1 comprise), minicarte daltonienne (bleu/vermillon, losanges ennemis seulement dans ce mode), repli de l'ordre du chef sur toute l'armée (émis directement sur le sim, l'issue décide), HUD masqué pendant le plan cinématique, pastilles d'état des cartes (déroute, ralliée, hésite, sous le feu, munitions, charge, mêlée, mode, tir, épuisée).
Restes : « Revoir la bataille », `ResultExtras` à brancher (flow / menu historique).

## bhud4 (vague 5)
Fait : toutes les touches du HUD passent par `KeyMap` (`battle_pause`, `battle_speed_up/down`, `battle_journal`, `battle_order_1..4`, `battle_burn`, `help_open`), dans des systèmes dédiés `.run_if(keyboard_free)` (les boutons souris restent actifs pendant une saisie). Libellés des ordres (infobulles, boutons) lus dans KeyMap. Aide F1 : modèle `HELP_TEXT` à `{action}` / `{action:1}` rempli depuis KeyMap et la disposition, reconstruite quand les touches ou la disposition changent (écart : l'aide cite aussi vue tactique et bannières). Bouton « Revoir la bataille » sur l'écran de résultat (message `WatchAgain`, seulement si le combat est enregistré). Preuve : `rebind_script.txt` (pause P, journal N, aide F9 : anciennes touches sans effet, nouvelles actives).
Restes : modificateurs Maj/Ctrl codés en dur (cards, minimap : clic, pas des actions) ; `ResultExtras` à brancher (flow / menu historique) ; `WatchAgain` : retour de la relecture vers le résultat non vérifié ici.
