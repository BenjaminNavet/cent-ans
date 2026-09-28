# OM-D5 — registre Anatolie, Caucase, Levant, Égypte (1337)

Branche `feat/om-d5`. Données seulement (pas de cargo, pas de géo, pas de Godot). Générées par un script jetable
(hors dépôt), contrôle d'intégrité perso OK (références, capitales possédées, colonies uniques, schémas, emprise EPSG:3035).

## État : TERMINÉ (en attente de fusion et de régénération géo)
48 provinces (regions `anatolie`, `caucase`, `levant`, `egypte`, `mesopotamie`), 147 colonies, 23 factions jouables,
23 titres, 20 personnages, 5 fichiers de noms (`names_tr/hy/ka/ady/os`) + `cul_arabic` ajouté à `names_ar`,
23 maisons, 23 fiches front-end, 3 aires de portraits (`anatolie`, `levant`, `caucase`), 5 régions dans
`data/rules/mercenaries.json` (`region_names`).

## Tests (pytest tools/tests)
Échecs attendus avant régénération géo : 48 `test_settlement_file_is_valid[prov_*]` (hors emprise 4096 actuelle),
`test_horizon::test_every_province_has_a_tile`, 3 erreurs de `test_settlement_graph`, et
`test_feudal_titles::test_lieges_exist...` (Alanie et Circassie ont pour liège `tit_golden_horde`, créé par D3, absent de
cette branche). Blasons tous distincts (`test_shields_are_distinct_and_masked` vert).

## Choix et écarts
- Sinope fusionnée dans le beylik de Candar (elle est candaroğlu depuis 1322) : pas de faction Sinope distincte.
- Ramadanides : absents (fondés vers 1352 ; Adana est encore arménienne en 1337). Dulkadir inclus (1337 ou 1348 selon les
  sources, incertain), vassal du sultan mamelouk.
- Chirvan, Tabriz, Bagdad : hors carte. Mossoul (dans la carte) est tenue par `fac_jalayirids` dont la capitale est
  replacée à Mossoul (Hasan Buzurg vraiment à Bagdad, hors carte). Mardin : Artuqides, vassaux mamelouks.
- Haute-Égypte : la carte s'arrête vers 27,2° N ; `prov_upper_egypt` s'arrête à Minya (Assiout est hors carte).
- Armée de Léon IV : Ayas tombe en 1337 (après le printemps), donc encore arménienne au départ.
- Factions sans souverain nommé : Karasi, Hamid, Teke, Circassie, Alanie (sources absentes ou incertaines).
- Vassalités écrites côté vassal (`suzerain`) ; la relation `overlord` de la Horde vers Circassie et Alanie est à
  ajouter dans `fac_golden_horde` (fichier D3) lors de la fusion.
- Dépendances : `cul_greek` (Trébizonde, `prov_trebizond`, `prov_kerasous`) et son fichier de noms sont à définir par D4 ;
  `fac_genoa`, `fac_venice` existent. Chypre, Rhodes, Byzance : D4.
- Marmara et Bosphore : pas de zone dédiée, `sea_aegean` (Kocaeli, Bursa, Mudanya).

## Doutes (à relire)
- Souverains à dates approximatives (marquées `uncertain`) : Saruhan, Menteşe (Orhan Bey), Germiyan (Yakub Ier),
  Karaman (Ibrahim), Eretna, Candar (Süleyman Paşa), Eşref (Mehmed), Dulkadir (Karaca), Beka Ier Djakéli, Hasan Buzurg,
  Salih d'Artuqide, al-Afdal de Hama, Umur (naissance), Basile de Trébizonde (naissance inconnue), Suleyman Paşa (Ottoman).
- Nicomédie prise en 1337 (sources ottomanes ; siège dès 1333) : `prov_kocaeli` ottomane, incertain.
- Konya karamanide, Ankara et Erzurum eretnides, Malatya mamelouke, Silifke karamanide, Ordu trapézontine : incertains.
- Anuk héritier désigné en 1337 : incertain (Abu Bakr est désigné en 1341 sur son lit de mort).
- Sites de Zikhia (Nikopsis, Maïkop) et d'Alanie (Alagir, Nuzal) approximatifs.
- Blasons : tous de convention (`uncertain`, `substitution`) ; croissant et lune non dessinables (grammaire actuelle).
- Dynasties de Hamid et Teke non identifiées en 1337.

## Relecture historienne
- Sûrs : Orhan (Bursa 1326, Nicée 1331), al-Nasir Muhammad (né le 24 mars 1285, mort le 7 juin 1341), Léon IV (1309-1341,
  Ayas 1337), Basile de Trébizonde (1332-1340, empoisonné le 6 avril 1340), Georges V (1314-1346), Anuk né le 8 avril 1323
  (mort en 1340), Umur (1309-1348), Abu l-Fida mort en 1331 et remplacé par son fils al-Afdal.
- Fragiles : tout ce qui est marqué `uncertain` ci-dessus. Anachronismes signalés : château de Bodrum (1406) et
  Vladikavkaz (1784) évités ou notés.
