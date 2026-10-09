# 0280 — Cartes d'unité, file de recrutement annulable et pastilles de la barre haute (WH uicards)

Statut : accepté (lot WH `uicards`).

## Contexte
Écarts d'interface relevés contre Total War: Warhammer III (`docs/wip/wh/ui.md` § 3) : expérience des régiments invisible, panneau de colonie sans population ni mécontentement, aucune prévision des pertes avant la bataille, file de recrutement en texte et non annulable, barre haute sans prestige/effectifs/troubles, pas de fiche d'unité au clic droit.

## Décision
- **Expérience (top1)** : le cœur expose déjà `unit.experience` (0-10, pas 0-9 comme dit le rapport). `data/ui/unit_ranks.json` (schéma `unit_ranks_ui`) porte le niveau maximal et les libellés de paliers ; `UnitRank` (GDScript) les lit. Un chevron par niveau en haut à droite de la carte, ligne « Expérience : n/10 (palier) » dans l'infobulle brute et riche.
- **Fiche d'unité (top10)** : `ArmyStrip.unit_details_requested(index)` au clic droit ; `HudController.show_unit_details` épingle l'infobulle riche existante (`CodexBubbles.pin_control_tooltip`) qui porte déjà attaque, défense, vitesse (`RichTooltip.unit_spec`). Pas de `get_unit_type_stats` : `GameCatalog` suffit.
- **Colonie (top4)** : `SettlementController` joint `province_state` au détail ; le panneau affiche Population et Mécontentement (`PanelWidgets.unrest_text`, partagé avec le panneau de province, mention de révolte comprise).
- **Pertes estimées (top5)** : `forecast_sides` cumule les pertes moyennes par camp (`attacker_losses_pct`, `defender_losses_pct`, % des effectifs) sur les mêmes tirages que la probabilité de victoire ; le dialogue d'avant-bataille ajoute « pertes estimées : vous x %, ennemi y % ».
- **File de recrutement (top6)** : `Order::CancelRecruit { settlement, index }`. Remboursement = prix payé (`QueuedRecruit.paid`, repli : coût de base) × `economy.json` `recruit_cancel_refund_percent` (100 % le tour même de l'ordre) ou `recruit_cancel_refund_late_percent` (50 % ensuite) ; l'emplacement est libéré, les ressources réservées relâchées, la réserve de recrues récupère l'unité. Le détail de colonie expose `recruit_queue_refund[]` ; le panneau montre une carte par recrue avec croix.
- **Barre haute (top7)** : `get_faction_summary` expose `prestige` (souverain), `soldiers` (effectif des armées) et `mean_unrest` (mécontentement moyen pondéré par la population, arrondi) ; trois pastilles dans `top_fit` (forme longue/courte), infobulles `chip_*` de `tooltips.json`.
- **Disposition (C2/C3 de `po_ui_test`)** : le bandeau d'ost s'arrête à gauche du panneau latéral (son bord droit est borné par `SIDE_PANEL`) ; les 14 px codés en dur (`campaign_map.tscn`, intitulés de jauges de province) passent par `UiType.CAPTION` (15 px).

## Conséquences
- Sauvegardes anciennes lisibles (`paid` et les deux taux ont des défauts).
- top3 (captifs à l'issue de la bataille : exécuter) non fait : l'effet de prestige et l'« ordre selon chivalry » ne sont pas définis (la chevalerie du dépôt est l'ordre de chevalerie, pas une vertu), choix de conception laissé à l'orchestrateur.
