# OM-D1 — Registre Nord et Baltique (1337)

Branche `feat/om-d1`. Périmètre : Danemark (interrègne), Suède-Norvège, Finlande, ordre Teutonique,
Livonie, Poméranie. **État : TERMINÉ** (données ; géo et Rust non lancés, hors mandat).

## Contenu
- 33 provinces nouvelles, 12 factions nouvelles jouables, 15 titres nouveaux, 12 personnages, ~115 colonies.
- Factions : `fac_holstein_kiel`, `fac_schleswig`, `fac_danish_estonia`, `fac_teutonic`, `fac_livonian_order`,
  `fac_riga_archbishopric`, `fac_riga`, `fac_dorpat`, `fac_osel_wiek`, `fac_courland`, `fac_pomerania_stettin`,
  `fac_pomerania_wolgast`. Norvège, Finlande, Gotland, Svealand… vont à `fac_sweden` (existante).
- Provinces : Danemark `north_jutland fyn schleswig skane harrien_wierland` ; Suède `ostergotland svealand
  bergslagen norrland gotland finland tavastia viborg` ; Norvège `viken vestlandet trondelag halogaland` ;
  Prusse `pomerelia kulm pomesania samland natangia galindia` ; Livonie `livonia courland pilten riga
  riga_archbishopric dorpat osel_wiek` ; Poméranie `pomerania_wolgast pomerania_stettin pomerania_stolp`.
- Régions nouvelles : `norvege finlande prusse livonie pomeranie` (ajoutées à `data/rules/mercenaries.json`).
- Cultures nouvelles : `cul_norwegian` (ajoutée à names_nordic), `cul_finnish` (names_fi), `cul_estonian` (names_et),
  `cul_latvian` (names_lv).
- Écus PNG des 12 factions et 8 maisons générés (`cent-ans assets heraldry`) ; pas de `.import` (Godot).

## Décisions de conception (à valider)
1. Couronne danoise vacante : `tit_denmark` (royaume, Jutland, Jutland du Nord, Fionie, Scanie) détenu par
   `fac_holstein` (Gérard III, régent et gagiste). `tit_holstein` (royaume fictif) supprimé, `fac_holstein.primary_title`
   = `tit_denmark`. Vassaux de jure sans hommage effectif : `tit_sjaelland` (Jean III de Holstein-Kiel), `tit_schleswig`
   (Valdemar V), `tit_estonia`. `prov_sjaelland` passe de `fac_holstein` à `fac_holstein_kiel`.
2. Norvège : pas de faction propre ; `tit_norway` tenu par `fac_sweden` (union personnelle) ; `tit_finland` (duché
   sous `tit_sweden`). Le champ `titles` de `fac_sweden` est inchangé.
3. Ermland (siège de Warmie vacant 1334-1337) fondu dans `prov_natangia` (Ordre) : pas de faction épiscopale.
4. Ordre livonien : `suzerain` = `fac_teutonic` ; Riga (ville) : `suzerain` = ordre livonien. Évêchés de Dorpat,
   Ösel-Wiek, Courlande : `de_jure_liege` = `tit_empire`, sans suzerain.
5. Colonies existantes déplacées : Oslo, Tønsberg (Götaland → Viken), Lund, Skanör (Götaland → Scanie), Aalborg
   (Jutland → Jutland du Nord), Løgumkloster (Jutland → Slesvig), Stralsund (Mecklembourg → Wolgast).
   Ajout de Varnhem à Götaland (elle n'aurait plus eu que 2 colonies). Ports retirés des trois provinces d'origine.
   `prov_jutland.voronoi_weight` 1.6 → 1.2 (Jutland du Nord prend sa part).
6. Blasons : brisures de jeu (bordure) pour Holstein-Kiel, Poméranie-Stettin, Wolgast, Ordre livonien pour que les
   écus soient distincts (le générateur ne dessine pas le griffon ni la feuille d'ortie) ; marqués `uncertain`.

## Points incertains (`uncertain`)
Populations (estimations) ; naissances de Dietrich, Eberhard, Friedrich, Engelbert, Jakob II, Johann II, Otto I,
Barnim III/IV, Bogislaw V ; Fionie aux mains de Gérard ; Halland suédois en 1337 ; évêque d'Ösel-Wiek Jakob II
peut être mort avant le printemps 1337 (successeur en 1338) ; capitaine d'Estonie danoise inconnu (faction sans
souverain) ; Riga sans chef nommé ; blasons épiscopaux non retrouvés ; Bogislaw V mineur/régence ; colonies
secondaires de Savonie (Lappee, Vehkalahti) et de Norrland approximatives.

## Relecture historienne
- Interrègne 1332-1340 : Jean III maître à l'est du Grand Belt, Gérard à l'ouest, Skåne vendue à Magnus (1332) : conforme.
- Valdemar V de Slesvig (1314-1364, duc depuis 1330, roi Valdemar III 1326-1329) : conforme aux sources consultées.
- Ordre : Dietrich von Altenburg 1335-1341, Marienbourg capitale depuis 1309, Pomérélie prise 1308-09 : conforme.
- Livonie : Eberhard von Monheim 1328-1340, Riga soumise en 1330, Friedrich von Pernstein 1304-1341, Engelbert de Dolen
  1331-1341, Johann II de Courlande 1332-1353 : conforme (Wikipédia) ; Jakob II d'Ösel-Wiek 1322-1337.
- Poméranie : Wolgast (Rügen 1325), Stettin (Otto I, Barnim III), suzeraineté brandebourgeoise levée en 1338.
- Anachronismes signalés : Kastelholm/Raseborg (châteaux en construction), cathédrale du Kneiphof (1333).

## Tests
`uv run --project tools pytest -q tools/tests` : 996 passed, 2 skipped, 15 failed + 3 errors, tous géo (hors emprise
de la carte actuelle ou tuiles/graphe à régénérer) : `test_horizon::test_every_province_has_a_tile`, 14
`test_settlements_schema::test_settlement_file_is_valid[...]` (colonies hors carte, lon > 16° E ou lat > 60° N),
3 erreurs `test_settlement_graph`. Script de contrôle (références, propriétaires, capitales possédées, bâtiments par
genre) : aucune erreur nouvelle.

## À faire par l'orchestrateur
Régénérer la géo ; `cargo test` (non lancé ici, compteurs figés de `sim-campaign/tests/campaign.rs` : +12 factions,
+33 provinces, armées) ; fiches horizon/décor de bataille (`data/fx/horizon.json`, `data/rules/battle_decor.json`)
pour les nouvelles provinces ; relations avec fac_poland, fac_lithuania, fac_novgorod, fac_pskov, fac_bohemia (D2/D3)
une fois créées : Ordre et ordre livonien en guerre avec la Lituanie, litige avec la Pologne ; ajouter Valdemar
Atterdag (prétendant, en exil) si un lot le prévoit.
