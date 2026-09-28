# OM-D4 — registre Balkans, Byzance, Égée (1337)

Branche `feat/om-d4` (issue de `feat/om`). Brief : scratchpad `brief-common.md` + `brief-data.md`.

## État : TERMINÉ (données ; régénération géo à faire par l'orchestrateur)
- 45 provinces (`prov_*`, régions `balkans`, `grece`, `egee`, `chypre`) et leurs colonies (3-5 chacune).
- 12 nouvelles factions jouables : `fac_byzantium`, `fac_epirus`, `fac_serbia`, `fac_bulgaria`, `fac_vidin` (vassal
  de la Bulgarie), `fac_bosnia`, `fac_ragusa` (vassale de Venise), `fac_durazzo` (vassale de Naples), `fac_athens`
  (vassale de la Sicile), `fac_archipelago`, `fac_hospitallers`, `fac_cyprus`.
- Provinces attribuées à des factions existantes : `fac_venice` (Crète, Négrepont, Modon, Zara, Spalato),
  `fac_taranto` (Achaïe, Corfou), `fac_genoa` (enclave de Galata, colonie `set_pera`). Relations ajoutées (ajouts
  seulement) à `fac_venice`, `fac_taranto`, `fac_naples`, `fac_sicily`, `fac_genoa`.
- 22 titres, 23 personnages, 12 maisons (`arms_of` faction), 12 fiches front-end, factions dans le bucket portraits
  `italy_empire` (provisoire : à déplacer vers un bucket orthodoxe en vague 3).
- Noms : `names_el` (cul_greek), `names_sr` (cul_serbian, cul_bulgarian, cul_bosnian), `names_sq` (cul_albanian),
  `names_dl` (cul_dalmatian).
- Fichiers partagés modifiés (ajouts en fin de liste) : `data/heraldry/houses.json` (+ `fac_athens` aux listes de la
  maison `barcelone_sicile`), `data/ui/front_end.json`, `data/portraits/archetypes.json`,
  `data/rules/mercenaries.json` (`region_names` : balkans, grece, egee, chypre).

## Validation
`uv run --project tools pytest -q` : 983 passed, 2 skipped, 40 failed, 3 errors. Les échecs sont tous liés à la géo
non régénérée : `test_settlements_schema` (39 fichiers : colonies hors de la grille actuelle), `test_settlement_graph` (3),
`test_horizon::test_every_province_has_a_tile`. Aucun autre échec. Script de contrôle maison : références, propriétaires,
capitales, titres (une seule appartenance par province), colonies (une cité par province) : OK. Pas de cargo (interdit).

## Relecture historienne / doutes (marqués `uncertain` dans les données quand possible)
- Épire : régence d'Anne Paléologine pour Nicéphore II au printemps 1337 (Jean II Orsini mort en 1335 ou 1337 selon les
  sources) ; annexion byzantine en 1338. Les dates de naissance des deux sont approximatives.
- Dalmatie : Zara vénitienne (sûr) ; Split, Trogir, Šibenik attribuées à Venise (statut 1337 à confirmer, surtout Šibenik).
- Durazzo : duc Charles (b. 1323) sous régence d'Agnès de Périgord depuis 1336 ; attribution à vérifier.
- Territoires byzantins : Kastoria, Serrès, Bérat/Avlona/Canina (statut exact 1337 incertain), Anchialos/Mésembrie.
- Bulgarie : Zagora et Philippopolis bulgares ; Dobroudja rattachée à la Bulgarie (Balik/Dobrotitsa autonomes, non modélisés).
- Vidin : Belaur, dates de règne inconnues, faction vassale. Héraldique de Vidin, Bulgarie, Bosnie, Ragusa, Archipel : conventions.
- Bosnie : rang `kingdom` par approximation (banat). Hospitaliers, Archipel : rang `county` sans suzerain.
- Achaïe et Corfou tenues par `fac_taranto` (Catherine de Valois-Courtenay, Robert de Tarente) faute de faction propre.
- Omis : Lemnos, Thasos, Céphalonie/Zante, Cilicie/Anatolie (D5), Belgrade et Croatie intérieure (D2), Valachie (D2).
- Populations : estimations par densité, marquées `uncertain`.
- Brisures de jeu : blasons distincts imposés par le test de distinction des écus (Durazzo, Hospitaliers).
