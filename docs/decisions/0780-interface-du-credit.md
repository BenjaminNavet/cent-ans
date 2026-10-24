# ADR 0780 — Interface du crédit et des impôts exceptionnels (lot CREDIT-UI)

Date : 2026-10-24. Statut : accepté. Complète l'ADR 0777 (le cœur livrait les ordres sans bouton) et l'ADR 0778 (budget lisible).

## Contexte

Le cœur sait emprunter, rembourser, lever l'aide, la gabelle et le fouage, et déclarer la banqueroute royale, mais le client n'affichait qu'un tableau de budget et une ligne « Dette aux banquiers » : le joueur ne pouvait rien décider.

## Décision

1. **Un onglet « Finances »** du panneau de faction (`campaign/panels/faction/finance.rs`, 5e onglet du joueur seul), ouvert par Maj+J (`campaign_toggle_finances`, déplaçable) ou par le bouton « Voir les finances » du conseiller. Il montre : trésor, solde prévu et trésor après la saison ; barres recettes / dépenses par poste (largeur proportionnelle, infobulle chiffrée : valeur, part du côté, saison passée) ; jauge de risque de banqueroute (bandes et repère d'alerte en données) ; dette par banquier avec taux, plafond restant et prêt minimal ; obligations ; emprunt, remboursement, impôts exceptionnels ; huit dernières saisons.
2. **Montants par paliers** (¼, ½, plafond pour l'emprunt, ¼, ½, tout pour le remboursement), pas de curseur : un curseur reconstruirait le panneau à chaque pas et un palier se clique par son nom (`faction:fin:borrow_step:<n>`) comme se lit un dump. Le palier fixe un pourcentage ; l'arrondi (50 ₶ vers le bas, jamais sous le prêt minimal ni au-dessus du plafond) est de la présentation, le cœur revalide tout.
3. **Ordres** par `game_api::diplomacy::submit_order` (`Borrow`, `Repay`, `LevyExtraordinary`), comme les autres panneaux ; aucune règle dans le client. Un refus du cœur s'affiche tel quel (« Refusé : … ») ; le dernier ordre est dans le dump.
4. **États grisés expliqués** : crédit gelé jusqu'au tour N, plafond atteint, plafond sous le prêt minimal, aucune dette, trésor vide, impôt en délai (« disponible dans N saisons »), impôt pas encore en usage (« n'existe pas avant 1341 ») ; la raison est écrite sous le bouton et dans son infobulle. Les impôts non encore disponibles sont lus dans `data.finance_rules.levies` (la vue du cœur ne liste que ceux de l'année).
5. **Textes en données** : `data/ui/finance_panel.json` (schéma `finance_panel.schema.json`) porte tous les textes, les paliers, les bandes de la jauge, le seuil d'alerte (60 %) et le nombre de saisons de l'historique.
6. **Conseiller** : au-delà du seuil de risque, un message permanent `finance:risk` (priorité Avertissement, action « Voir les finances ») ; retiré quand le risque retombe. Une banqueroute (compteur `finance.defaults` d'une faction qui augmente) est annoncée au bandeau : urgente pour le joueur, simple information pour une autre faction.
7. **Focus** (ADR 0753/0775) : `FocusDefault` sur le banquier choisi (action sans engagement) ; les boutons gardent l'ordre de lecture. Dump `finance_panel` (valeurs, banquiers, boutons actifs/grisés, barres, dernier ordre).

## Conséquences

- Aucun changement du cœur. Un seul nouveau raccourci (Maj+J).
- L'infobulle d'une barre est en texte brut du côté client, sans lien Codex.
- Les montants intermédiaires libres (curseur) restent possibles plus tard sans toucher au cœur.
