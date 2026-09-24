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
| france_est | prov_bar | à faire |
| france_est | prov_bourgogne | à faire |
| france_est | prov_champagne | à faire |
| france_est | prov_franche_comte | à faire |
| france_est | prov_lyonnais | à faire |
| france_nord | prov_artois | à faire |
| france_nord | prov_boulonnais | à faire |
| france_nord | prov_ile_de_france | à faire |
| france_nord | prov_normandie | à faire |
| france_nord | prov_normandie_ouest | à faire |
| france_nord | prov_picardie | à faire |
| france_nord | prov_ponthieu | à faire |
| france_ouest | prov_anjou | à faire |
| france_ouest | prov_bretagne | à faire |
| france_ouest | prov_bretagne_ouest | à faire |
| france_ouest | prov_maine | à faire |
| france_ouest | prov_poitou | à faire |
| languedoc | prov_beaucaire | à faire |
| languedoc | prov_carcassonne | à faire |
| languedoc | prov_montpellier | à faire |
| languedoc | prov_rouergue | à faire |
| languedoc | prov_toulousain | à faire |
| provence_alpes | prov_comtat_venaissin | à faire |
| provence_alpes | prov_dauphine | à faire |
| provence_alpes | prov_provence | à faire |
| provence_alpes | prov_savoie | à faire |

## Enclaves posées jusqu'ici

- `set_aiguillon` (Agenais) — château anglais, owner `fac_england`.
- `set_castelnaud` (Périgord) — château anglo-gascon, owner `fac_england`.
- `set_bergerac` (Périgord) — ville close relevant du duché anglais avant sa reprise française de 1345, owner `fac_england`.
- `set_pons` (Saintonge) — forteresse au sud de la Charente relevant du duché d'Aquitaine anglais jusqu'à la confiscation du 24 mai 1337, owner `fac_england`.

## Prochaine étape

Continuer avec `france_est` (5 provinces), puis `france_nord`, `france_ouest`, `languedoc`, `provence_alpes`.
Commit `wip:` toutes les ~8 provinces.
