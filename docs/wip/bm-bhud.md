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
