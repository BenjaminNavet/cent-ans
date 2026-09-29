# OM D6 — Maghreb et compléments méditerranéens (1337)

Branche `feat/om-d6` (worktree `../gp-om-d6`), issue de `feat/om`. **État : TERMINÉ (données), reste l'intégration géo.**

## Contenu
- 6 factions jouables : `fac_marinids`, `fac_hafsids`, `fac_makki` (Gabès et Djerba), `fac_tripoli` (Banu Thabit),
  `fac_barqa` (tribus de Cyrénaïque), `fac_arborea` (Sardaigne, vassale d'Aragon). 6 titres, 6 personnages,
  fiches front-end, 4 maisons, noms `names_ma` (cul_maghrebi) et `names_be` (cul_berber), cultures `cul_sardinian`
  (ajoutée à `names_it`).
- 37 provinces, 120 colonies (Maghreb 29 dont Malte 1 ; Italie/Sardaigne 7 : Abruzzes, Capitanate, Terre de Bari,
  Basilicate, Calabre citérieure et ultérieure, Arborée).
- Modifs de fichiers existants (ajouts) : `tit_naples` (+6 provinces), `tit_sicily` (+prov_malta), `names_it`,
  `houses.json`, `front_end.json`, `portraits/archetypes.json` (buckets iberia/italy_empire, provisoire),
  `rules/mercenaries.json` (région `maghreb`), `settlements/prov_sicilia.json` (+Pantelleria),
  `settlements/prov_sardegna.json` (Oristano et Bosa déplacées vers `prov_arborea`), `codex/cdx_abu_al_hasan_ali.json`.

## Choix
- Départ = printemps 1337 ; Tlemcen tombe le 1er mai 1337 : traitée comme déjà mérinide, aucune faction zayyanide.
  Oranie et Alger (contrôle incertain) mérinides.
- Algésiras et Gibraltar : nouvelle `prov_algeciras` mérinide (Tarifa : enclave castillane, `owner: fac_castile`).
  Ronda et Marbella restent grenadines (déjà dans `prov_malaga`, pas de doublon).
- Barqa : faction tribale autonome plutôt que province mamelouke (les Mamelouks n'y exercent qu'une suzeraineté
  nominale). Aucune référence à `fac_mamluks` codée : lien nominal à ajouter à l'intégration après D5.
- Tripoli : faction propre (Muhammad ibn Thabit, quasi indépendant) ; Gabès : Banu Makki, vassaux hafsides, sans
  souverain nommé (nom de l'émir de 1337 non établi). Djerba : colonie de `prov_gabes`, `owner: fac_hafsids`
  (reprise en 1335).
- Malte et Gozo : une province (`prov_malta`, fac_sicily, culture `cul_sicilian`) ; Pantelleria : simple colonie de
  `prov_sicilia` (île trop petite pour une province ; elle sera hors du Voronoï terrestre, à surveiller).
- Corse et Sicile : pas de scission (déjà couvertes par une province chacune).
- Portraits : aucun bucket islamique ; factions musulmanes rangées provisoirement dans `iberia` (à remplacer en vague 3).

## Points incertains (`uncertain`)
Naissance d'Abu Bakr II (vers 1290), d'Ibn Thabit (1290), de Pierre III d'Arborée (vers 1300), d'Abu l-Hasan (1297) et
d'Abu Inan (1329) approximatives ; héritiers non désignés (Abu Inan n'est pas établi comme héritier en 1337) ;
mariage mérinide-hafside ; contrôle mérinide à Alger et dans l'Oranie ; autorité hafside sur les Ziban et le
Nefousa ; Sirte, Benghazi (peu de peuplement) ; populations toutes estimées ; blasons de jeu (aucun blason attesté
sauf Arborée, dont l'arbre n'est pas rendu). L'ancienne description d'Oristano (« résistance ouverte à l'Aragon »)
a été remplacée : Pierre III est allié de la couronne (à revérifier).

## Relecture historienne
Cohérent avec les sources consultées : Tlemcen (mai 1337), Gibraltar (1333), Djerba (1335), Abu Bakr II (1318-1346),
Ibn Thabit à Tripoli (v. 1326-1348), Banu Makki à Gabès (depuis 1282). Douteux : fac_arborea vassale (Mariano IV se
révolte en 1353) ; nom `chr_abu_bakr_ii_hafside` OK ; l'autorité tripolitaine sur le Nefousa.

## Validation
`uv run --project tools pytest -q` : 996 passed, 2 skipped, 19 failed + 3 errors, tous liés à la géo non régénérée
(colonies hors de la grille 4096 : `test_settlement_file_is_valid[...]` pour les provinces au sud de lat 35 ;
`test_horizon::test_every_province_has_a_tile` ; `test_settlement_graph` ×3). Vérifs à faire après régénération :
les colonies tombent dans leur province (simulateur planaire local : seuls Reggio, sur Sicile, et Andernach
hors périmètre discordent).

## Reste (orchestrateur)
Régénérer la géo ; fusion 3 voies (`houses.json`, `front_end.json`, `archetypes.json`, `mercenaries.json`,
`tit_naples`, `tit_sicily`, `names_it`) ; relier Barqa à `fac_mamluks` ; contrôler `cargo test` (compteurs figés
`new_1337_matches_game_data`).
