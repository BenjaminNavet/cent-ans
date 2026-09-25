# 0027 — Météo de la carte de campagne : tirée au cœur, déterministe, visuelle pour l'instant

Date : 2026-09-25 (lot CM2, carte de campagne : vue parchemin et météo)

## Contexte
La carte de campagne doit montrer pluie, neige, brouillard matinal et orages qui passent selon la
saison et la région (backlog TW, « Carte de campagne »). Jusqu'ici, rien ne décrivait le temps sur
la carte : l'ambiance sonore (AU1) tirait une météo au hasard par saison, et la bataille tire la
sienne dans `battle_auto::draw_weather` (flux aléatoire de la campagne). Toute règle vit dans
`core/` ; `game/` ne fait que le rendu.

## Décision
- **Au cœur, sans état** : `sim-campaign/src/weather.rs` calcule la météo de chaque province pour
  le tour courant, fonction pure de la **graine** de la partie, du **tour** (et de sa saison) et de
  la **position de la capitale** (`geo.capital_lonlat`). Rien n'est sauvegardé, le flux aléatoire
  de la campagne n'est pas consommé : sauvegardes, rejeu et IA sont inchangés.
- **Fronts** : un champ de bruit régional (taille `front_scale_deg`, qui dérive vers l'est de
  `drift_deg_per_turn` à chaque tour et change de forme d'un tour à l'autre) plus un peu de bruit
  propre à la province (`local_jitter`) classe les provinces du plus sec au plus humide ; chaque
  province lit son rang dans les chances de son **climat** (océanique, continental,
  méditerranéen, montagnard) et de la **saison** (`data/rules/campaign_weather.json`, schéma
  `campaign_weather_rules.schema.json`). Le classement fait tenir exactement les chances sur la
  carte tout en gardant des régions entières sous le même ciel. Genres : clair, brouillard, pluie,
  neige, orage ; intensité [0, 1] dans le genre.
- **Pont** : `CampaignSim.get_campaign_weather()` (`{province: {kind, intensity, label}}`) et
  `get_province_weather(id)`.
- **Effet de jeu : aucun pour l'instant.** La météo est rendue sur la carte (nuées, pluie, neige,
  brouillard, éclairs) et entendue (AU1 lit cette source au lieu de son tirage). Les batailles
  gardent leur propre tirage (N1) pour ne pas modifier l'équilibrage ni les tests de calibrage.

## Conséquences et suites possibles
- Brancher un effet de règle est local : `battle_auto` pourrait prendre la météo de la province
  de la bataille au lieu de `draw_weather` (pluie → arcs gênés, déjà modélisé), la marche pourrait
  ralentir sous la neige. Chaque effet devra être équilibré (sondes G1) et décrit ici.
- Le rendu ne doit jamais inventer de météo : il lit le pont, et retombe sur « clair » sans
  campagne (maquette de simulation).
