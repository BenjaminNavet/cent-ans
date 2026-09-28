# OM-D3 — registre Rus' et Horde d'Or (1337)

Branche `feat/om-d3`. Données seulement (pas de cargo, pas de géo).

## État
- Données écrites (générateur jetable dans le scratchpad, données seules commitées) : 43 provinces, 129 colonies,
  20 titres, 18 factions jouables, 12 personnages, 3 fichiers de noms (ru, tt, fu), 8 maisons, 18 fiches front-end,
  2 aires culturelles de portraits (`rus`, `horde`), 6 régions ajoutées à `data/rules/mercenaries.json`.
- Contrôle d'intégrité perso : OK (références, capitales possédées, colonies uniques). Blasons tous distincts.

## Prochaine étape
Relecture historienne, rapport à l'orchestrateur (attente de la fusion et de la régénération géo).

## Faits vérifiés (web)
- Tver au printemps 1337 : Constantin Mikhaïlovitch (1327/28-1338, puis 1339-1346). Alexandre Mikhaïlovitch règne à
  Pskov jusqu'en 1337, va à la Horde en 1337 (pardon d'Özbeg), rentre à Tver à l'automne 1338, exécuté à Saraï
  avec son fils Fiodor le 28 (ou 29) octobre 1339. Il est donc ici prince de Pskov (incertain).
- Ivan Ier Kalita : prince de Moscou 1325, grand-prince de Vladimir 1331, mort le 31 mars 1340 ; Siméon né le 7 sept. 1316.
- Özbeg : 1282-1341, khan depuis 1313, fils Tini Beg puis Djanibeg.
- Riazan : Ivan II Korotopol 1327-1342. Iaroslavl : Vassili 1321-1345. Smolensk : Ivan Alexandrovitch (dep. 1313).

## Choix de modélisation
- Horde = titre `kingdom` (`tit_golden_horde`) ; Novgorod, Vladimir, Tver, Riazan, Souzdal, Smolensk = duchés sous la Horde ;
  Pskov, Perm, Viatka = comtés sous Novgorod ; Rostov, Iaroslavl, Beloozero = comtés sous `tit_vladimir` (orbite de Moscou).
  Briansk, Haute-Oka, Mordves, Bachkirs, Théodoro, Gazarie (Caffa, tenue par `fac_genoa`) = comtés sous la Horde.
- Provinces directes de la Horde : Saraï, Hadji-Tarkhan, Bolgar, steppes Don/Dniepr, Boudjak, Saraïtchik, Kouban, Azaq
  (Tana : colonie `owner: fac_venice`), Crimée. Caffa est à `fac_genoa`.
- Dépendances hors lot : cultures `cul_ruthenian` (Tchernigov, dans `names_ru`, à confirmer avec D2) et `cul_greek`
  (Théodoro, à définir par D4 avec son fichier de noms).

## Doutes (à relire)
- Tver au printemps 1337 : Constantin (source : Wikipédia). Alexandre à Pskov jusqu'en 1337 puis à la Horde : position exacte
  au printemps incertaine. Retour à Tver automne 1338, exécution 28 (ou 29) octobre 1339 (choix : 28).
- Naissances approximatives (Ivan Korotopol, Constantin de Souzdal, Vassili de Iaroslavl, Ivan de Smolensk, Dmitri de Briansk,
  Fiodor Danilovitch, Tini Beg, Constantin de Tver) : marquées `uncertain`.
- Pas de souverain nommé : Rostov, Beloozero, Haute-Oka, Mordves, Bachkirs, Perm, Viatka, Théodoro (sources absentes).
- Localisations approximatives des campements de steppe, de Mokhcha, Bilyar, Joukotine, Saraï al-Djadid, Hadji-Tarkhan.
- Rattachements incertains : Tchernigov à Briansk, Mourom à Souzdal, Ouglitch/Galitch/Kostroma à Moscou, Vologda à Beloozero.
- Blasons : tous de convention (`uncertain`, `substitution`) ; les Rus' de 1337 n'ont pas d'armoiries fixées.
- Ordre de succession de la Horde : `elective` (kurultaï) par défaut, incertain.

## Tests (pytest complet, 2026-09-28)
977 passed, 44 failed, 3 errors, tous attendus avant régénération géo : 41 `test_settlement_file_is_valid[prov_*]`
(« off the map » : hors de l'emprise 4096 actuelle), `test_horizon::test_every_province_has_a_tile`,
3 erreurs de `test_settlement_graph` (graphe non régénéré). Les deux échecs réels (nom de capitale ≠ nom de la cité)
ont été corrigés (Mokhcha, Oufa).

## Relecture historienne
- Sûrs : Özbeg (1282-1341, khan 1313), Ivan Kalita (mort le 31 mars 1340), Siméon (1316), Alexandre de Tver (né 7 oct. 1301,
  exécuté à Saraï en 1339), Vassili Kalika archevêque, concession vénitienne de Tana (1332-33), Caffa génoise depuis 1266.
- Fragiles : Novgorod-Ivan Kalita « prince jusqu'en 1337 » ; le tribut de Novgorod passe par le grand-prince (d'où le suzerain
  Horde) ; Smolensk sous suzeraineté de la Horde (1337) plutôt dans l'orbite lituanienne ; Boudjak sans Moldavie ni Valachie ;
  Caffa/Kertch : Cerco génois attesté mais statut 1337 flou ; Soldaïa génoise seulement en 1365 (laissée à la Horde) ;
  Théodoro sans prince nommé ; Moscou/Ouglitch/Galitch acquis vers 1328-1340.
- Anachronismes signalés : Serge de Radonège (monastère vers 1337-1342), Kholmogory (1353), Khlynov (1374), Kalouga (1371),
  Oufa (1574, donc « site »).
