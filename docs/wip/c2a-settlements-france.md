# C2a : colonies de 1337 — France (7 régions, 41 provinces)

Spec : `docs/design/2026-09-24-echelle-colonies.md` § 3.1. Schéma : `data/schemas/settlement.schema.json`.
Régions couvertes : `france_nord`, `france_centre`, `france_est`, `france_ouest`, `aquitaine`, `languedoc`, `provence_alpes`.

Validation : `uv run --project tools python <script ad hoc>` (jsonschema + registry common.schema.json, règles métier :
une city = capital_city, 3-6 colonies, buildings/factions existants, province == nom de fichier, port si coastal).

## État par province

| Région | Province | État |
|---|---|---|
| aquitaine | prov_agenais | fait (5) |
| aquitaine | prov_angoumois | fait (5) |
| aquitaine | prov_bearn | fait (4) |
| aquitaine | prov_gascogne | fait (4) |
| aquitaine | prov_guyenne | fait (5) |
| aquitaine | prov_perigord | fait (5) |
| aquitaine | prov_quercy | fait (5) |
| aquitaine | prov_saintonge | fait (5) |
| france_centre | prov_auvergne | fait (5) |
| france_centre | prov_berry | fait (4) |
| france_centre | prov_bourbonnais | fait (4) |
| france_centre | prov_limousin | fait (5) |
| france_centre | prov_nivernais | fait (5) |
| france_centre | prov_orleanais | fait (5) |
| france_centre | prov_touraine | fait (5) |
| france_est | prov_bar | fait (4) |
| france_est | prov_bourgogne | fait (5) |
| france_est | prov_champagne | fait (6) |
| france_est | prov_franche_comte | fait (5) |
| france_est | prov_lyonnais | fait (5) |
| france_nord | prov_artois | fait (5) |
| france_nord | prov_boulonnais | fait (4) |
| france_nord | prov_ile_de_france | fait (6) |
| france_nord | prov_normandie | fait (5) |
| france_nord | prov_normandie_ouest | fait (5) |
| france_nord | prov_picardie | fait (5) |
| france_nord | prov_ponthieu | fait (4) |
| france_ouest | prov_anjou | fait (5) |
| france_ouest | prov_bretagne | fait (5) |
| france_ouest | prov_bretagne_ouest | fait (5) |
| france_ouest | prov_maine | fait (4) |
| france_ouest | prov_poitou | fait (5) |
| languedoc | prov_beaucaire | fait (5) |
| languedoc | prov_carcassonne | fait (5) |
| languedoc | prov_montpellier | fait (4) |
| languedoc | prov_rouergue | fait (5) |
| languedoc | prov_toulousain | fait (5) |
| provence_alpes | prov_comtat_venaissin | fait (4) |
| provence_alpes | prov_dauphine | fait (4) |
| provence_alpes | prov_provence | fait (5) |
| provence_alpes | prov_savoie | fait (5) |

## Enclaves posées jusqu'ici

- `set_aiguillon` (Agenais) — château anglais, owner `fac_england`.
- `set_castelnaud` (Périgord) — château anglo-gascon, owner `fac_england`.
- `set_bergerac` (Périgord) — ville close relevant du duché anglais avant sa reprise française de 1345, owner `fac_england`.
- `set_pons` (Saintonge) — forteresse au sud de la Charente relevant du duché d'Aquitaine anglais jusqu'à la confiscation du 24 mai 1337, owner `fac_england`.
- `set_besancon` (Franche-Comté) — cité impériale libre, distincte du comté de Bourgogne, owner `fac_empire`.
- `set_evreux` (Normandie/Rouen) — comté navarrais de Philippe d'Évreux, owner `fac_navarre`.
- `set_mortain` (Normandie/Caen) — comté cédé à Jeanne II de Navarre en 1336, owner `fac_navarre`.
- `set_avignon` (Comtat Venaissin) — la ville même relève du comte de Provence (Naples) jusqu'à son rachat en 1348 par la papauté, owner `fac_naples` (contraste avec `owner` de la province, `fac_papacy`, qui tient le reste du Comtat).

## Tâche C2a terminée

41 provinces, 196 colonies. Répartition : 41 city, 58 town, 38 castle, 37 abbey, 22 village.
Validation OK (schéma + règles métier : une city par province avec nom/coordonnées/bâtiments/fortification
identiques à `capital_city`/province, 3-6 colonies, ids uniques globalement, buildings et factions
existants, province == nom de fichier, port pour chaque province côtière).

## Prochaine étape

C2a fait. Prochaine étape pour l'orchestrateur : fusionner avec C2b (Nord) et lancer C2c (Sud),
puis C3 (pipeline géo, graphe des colonies).
