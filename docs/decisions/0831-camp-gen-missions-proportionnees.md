# 0831 — CAMP-GEN : missions proportionnées à la puissance de la maison

Statut : accepté (lot E de la boucle juge 2, itération 3 ; constats M1, M2, M5)

## Contexte
Le générateur de missions (ADR 0127) tire ses cibles de la carte seule. Il a offert « Prendre Yorkshire » à l'Écosse (100 hommes, 800 ₶ de revenu) et des levées de trois unités à la Navarre (369 ₶) qui échouaient par l'échéance et coûtaient du prestige. Les petites maisons n'avaient aucune mission tenable.

## Décision
- Une offre n'est tirée que si la maison peut la tenir. Les règles vivent dans `data/missions.json` § `scale` (schéma `missions.schema.json`) et le code dans `sim-campaign/src/mission_scale.rs`.
  - Conquête (`control`, modèles et rancunes) : hommes sous les armes >= `min_men_for_conquest` et force de la cible (garnisons du contrôleur + armées abritées dans la province) <= hommes × `max_target_force_ratio`.
  - Bataille (`battle_won`) : hommes >= `min_men_for_battle`.
  - Levée (`units_recruited`) : le nombre demandé est plafonné à ce que `levy_treasury_share` du trésor paie à l'unité la moins chère ; zéro : pas d'offre.
- Le test `missions_scaled` génère les offres des huit maisons à 1337 et vérifie la proportion ; `probe_offers(.., scaled=false)` mesure ce que le filtre retire.
- Les Affaires « Armée ennemie près de X » relèvent de l'agenda (lot XP-SOBRE) : hors périmètre.

## Conséquences
- Les maisons faibles voient moins d'offres de conquête, plus de missions de bâtir, de traité ou de défense ; la table de gabarits reste inchangée.
- Les seuils sont des réglages de données, pas des règles codées en dur.
