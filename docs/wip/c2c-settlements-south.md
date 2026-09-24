# C2c — colonies d'Ibérie et d'Italie

Tâche : `docs/design/2026-09-24-echelle-colonies.md` § 3.1. 35 provinces (régions
`iberie_centre`, `iberie_nord`, `iberie_sud`, `aragon`, `portugal`, `italie_centre`,
`italie_nord`, `italie_sud`).

## État : terminé

35/35 provinces écrites, 145 colonies (35 city + 58 town + 30 castle + 17 abbey +
5 village). Validation schéma + règles (une city par province au nom du
`capital_city`, 3-6 colonies, ids uniques, bâtiments/factions existants, coastal ⇒
≥1 port) : `uv run --project tools pytest -q tools/tests/test_settlements_schema.py`
→ 134 passed.

Provinces enrichies (5-6 colonies) : Milanais (6), Toscane (6), Vénétie (5),
Catalogne (5), Andalousie/Séville (5), Naples (5). Provinces pauvres (3
colonies, lieux documentés limités) : Sardaigne, Corse, Grenade, Majorque,
Roussillon.

Enclaves (`owner` non nul, vérifiées par recherche) :
- `set_lucca` (prov_firenze) → `fac_verona` : Lucques conquise en 1335 par
  Mastino II della Scala, seigneur de Vérone, tenue jusqu'en 1341.
- `set_treviso` (prov_venezia) → `fac_verona` : Trévise prise par Cangrande
  della Scala en 1329, cédée à Venise seulement en 1339 (après notre date).
- `set_mantova` (prov_ferrara) → `fac_empire` : Mantoue, seigneurie Gonzague
  reconnue vicariat impérial par Louis IV de Bavière en 1329, distincte du
  vicariat pontifical de Ferrare.
- `set_alghero` (prov_sardegna) → `fac_genoa` : place forte de la famille
  génoise Doria, fondatrice de la ville (~1102), tenue jusqu'à sa prise par
  l'Aragon en 1353.
- `set_peniscola` (prov_valencia), `set_calatrava_la_nueva` (prov_toledo),
  `set_alcantara` (prov_extremadura) : commanderies des ordres militaires
  (Montesa, Calatrava, Alcántara) classées `kind: "abbey"` conformément à la
  consigne de la tâche, `owner: null` (les ordres n'ont pas de faction dédiée
  dans `data/factions/`).

Points de doute résolus par recherche web (voir sources dans les fichiers) :
Gibraltar (tenu par les Mérinides en 1337, aucune faction disponible → omis) ;
Padoue (seigneurie autonome de Marsilio da Carrara depuis juillet 1337, alliée
à Venise mais pas encore annexée → `owner: null`, reste rattachée à la
province Venise par défaut, nuance dans la description) ; Alcalá la Real
(conquise par la Castille seulement en 1341 → remplacée par Almodóvar del Río
pour éviter l'anachronisme) ; l'épithète « œil droit de Grenade » appartient
à Íllora et non à Alhama de Granada (corrigé) ; le fuero de Saint-Sébastien
(1180) est navarrais, la ville ne passe à la Castille qu'en 1200 (corrigé).

## Prochaine étape

Tâche C2c terminée. Voir `docs/wip/colonies.md` pour la suite de
l'orchestration (C3 pipeline géo, C4 refonte cœur).
