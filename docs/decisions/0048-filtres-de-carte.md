# 0048 — Filtres de la carte de campagne

- Statut : accepté (25/09/2026, lot MF1)

## Contexte
La carte de campagne avait quatre « modes » éparpillés : mécontentement (booléen dans
`campaign_map.gd`), diplomatie et religion (`enum` dans `DiplomacyController`), routes commerciales
(calque). Les teintes pouvaient se superposer, seuls les routes avaient un bouton, et la touche R
servait à la fois la religion et les routes (la diplomatie consommait l'événement : les routes
n'étaient plus accessibles au clavier). Le mode mécontentement saturait au rouge (mécontentement
0-100 non ramené à 0-1). Il manquait les filtres attendus d'un Total War : richesse, population,
loyauté des vassaux, ravitaillement, revendications.

## Décision
- **Un seul mode de teinte actif** porté par `MapModeController` (`game/scripts/map/`) : politique,
  diplomatie, religion, mécontentement, richesse, population, loyauté, ravitaillement,
  revendications. Même touche deux fois = retour à la carte politique.
- **Les routes commerciales restent un calque**, combinable avec n'importe quel mode (touche V).
- **Un bouton « Filtres » (touche F)** en tête de la rangée de la minicarte ouvre le menu ; l'ancien
  bouton « Commerce » devient une case de ce menu.
- **Les valeurs viennent du cœur** : `sim_campaign::map_lens` (une passe pour toutes les provinces,
  les revendications de chaque faction rassemblées une fois) exposé par `CampaignSim.get_map_lens`.
  Le ravitaillement réutilise `economy::seasonal_supply_change`, extrait de `resolve_attrition` :
  la carte montre exactement la règle appliquée aux armées (sans l'effet d'un général).
- Richesse et population sont teintées **par rang** (percentile), pas par valeur : Paris écraserait
  sinon toutes les autres provinces.

## Conséquences
- `DiplomacyController` ne gère plus que le panneau et les propositions.
- Ajouter un filtre = un champ dans `ProvinceLens` et une entrée dans `MapModeController.MODES`.
- Richesse = impôt de base (`province_income`) : ne tient compte ni du taux d'imposition ni des
  bâtiments, pour comparer les terres elles-mêmes.
