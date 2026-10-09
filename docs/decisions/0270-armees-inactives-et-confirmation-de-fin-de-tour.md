# 0270 — Armées inactives, alertes d'oubli et confirmation de fin de tour

## Contexte
Revue WH (Total War: Warhammer III) : sur 464 tours, le joueur oublie une armée sans ordre, un
emplacement de construction libre ou la recherche. La condition « armée sans ordre » existait en
trois copies divergentes (confirmation de fin de tour, conseil `NextHint` qui exigeait une armée n'ayant
pas bougé du tout, plaque absente). L'alerte « recherche inactive » avait été retirée de la cloche alors
que le conseil `research_idle` la lit. Les alertes `enemy_army` listaient toute armée ennemie
du cœur, y compris hors de vue (fuite du brouillard : effectif d'une armée invisible).

## Décision
- Prédicat unique `CampaignAlerts.army_is_idle(army)` : ni chemin ni marche planifiée, mouvement restant,
  hors siège et hors embarquement ; `idle_armies(map)` en liste les identifiants. Utilisé par la cloche
  (alerte `idle_army`, `idle:<armée>`), `NextHint`, la plaque d'armée (statut « inactive », joueur seul)
  et la confirmation.
- Alertes `free_slot` (une seule, groupée : colonies sans chantier avec un bâtiment constructible au prix du
  trésor, d'après `get_holdings_overview`) et `research_idle` (aucune recherche et points > 0) rétablie.
- Brouillard : `enemy_army` passe par `MinimapController.is_army_visible`.
- Réglage `interface/confirm_end_turn` : `off | warnings | always`, défaut `warnings` (confirmation s'il y a au
  moins une alerte `idle_army / free_slot / research_idle`, liste dans le dialogue, « Voir » ouvre la
  première). Migration de l'ancien booléen : vrai → `always`, faux → `warnings` (l'ancien « faux » est la
  valeur par défaut écrite dans tous les fichiers de réglages, pas un choix). Les tests (`use_test_file`)
  démarrent en `off`.
- Touches (InputMap, fiche et réaffectation via `ShortcutSheet.CAMPAIGN_SECTIONS`) : Tab / Maj+Tab armée
  inactive suivante / précédente (centre et sélectionne), Ctrl+Tab colonie suivante, Début capitale,
  Maj+Entrée fin de tour sans confirmation, `.` centrer, X séparer, H garnison, 1 à 6 postures.
  Correspondance exacte des modificateurs (Tab ≠ Maj+Tab). Logique dans `CampaignHotkeys` /
  `FlowController.handle_hotkey`.

## Conséquences
Les tests qui terminent le tour sur une vraie carte ne subissent pas de dialogue (`off` en test). Le réglage
n'a pas encore d'option par type d'alerte. Le nouveau couplage plaque → `CampaignAlerts` est local à l'interface.
