# OM-D2 — registre Europe centrale et orientale (1337)

Branche `feat/om-d2`. Fichiers produits par un générateur jetable (hors dépôt) : ce sont les JSON qui font foi.

## État : TERMINÉ (données)
- 51 provinces nouvelles (`prov_*`), 15 factions jouables, 15 titres, 22 personnages, ≈ 110 colonies,
  6 fichiers de noms (`names_pl/hu/lt/ro/sh/sk`), 12 maisons héraldiques, 15 fiches front-end.
- Contrôle maison (`scratchpad/d2/check.py`) : schémas, références, une cité par province, ids de colonies
  uniques, capitale possédée par sa faction : aucun défaut sur les fichiers D2.
- `pytest` complet : 1001 passés ; échecs attendus (géo pas régénérée, hors D2) : voir « Tests rouges ».

## Contenu
- Pologne (fac_poland) : Grande-Pologne, Kuyavie, Sieradz-Łęczyca, Petite-Pologne, Sandomierz, Lublin.
- Mazovie, vassale de la Bohême (hommage 1329) : Płock (Boleslas III, mineur, tutelle de ses oncles), Czersk-Varsovie
  (Trojden Ier), Rawa (Siemowit II).
- Silésie : Wrocław (couronne de Bohême, fac_bohemia), Legnica-Brzeg, Głogów-Żagań, Haute-Silésie (Opole, Cieszyn,
  Opava), Świdnica-Jawor indépendant. Moravie (`prov_moravia`, fac_bohemia).
- Habsbourg : `prov_styria`, `prov_carinthia`, `prov_carniola` ; `prov_austria` reseedée en (15.3, 48.2) poids 1.4,
  `set_graz` déplacé vers `prov_styria`.
- Hongrie (10 provinces royales) + voïvodie de Transylvanie (2) + banat de Slavonie et Croatie intérieure (3).
- Galicie-Volhynie (4), Lituanie (10), Kiev (fac_kiev, indépendante), Valachie (3).

## Écarts et incertitudes (à relire)
- **Kuyavie** : occupée en 1337 par l'Ordre teutonique (Kalisz 1343 la restitue) ; rangée chez la Pologne selon le brief. À
  coordonner avec D1 (ne pas la donner deux fois).
- Mazovie : vassalité bohémienne de 1329 sûre pour Wenceslas de Płock et attestée pour Rawa ; incertaine pour Trojden.
  Titres de rang `county` sous `tit_bohemia` (rang `duchy`) car le schéma n'autorise que trois niveaux.
- Głogów-Żagań : Henri IV de Żagań (père d'Henri V le Fer) ; sa vassalité n'est attestée qu'en 1344, donc `uncertain`.
- Rang `kingdom` approximé pour les souverains non royaux (Świdnica, Kiev, Valachie, Lituanie), comme Florence/Venise.
- Dates de naissance/mort marquées `uncertain` : Aldona, Élisabeth, Szécsényi, Mikcs Ban, Iouri II (naissance), Trojden,
  Siemowit II, Boleslas III de Płock, Gediminas, Algirdas, Kęstutis, Jaunutis, Fiodor de Kiev, Basarab, Nicolas Alexandre,
  Henri IV de Żagań, Boleslas II d'Opole, Bolko II de Świdnica (naissance). Mikcs Ban : bans de Slavonie/Croatie à vérifier.
- Vitebsk, Polotsk, Minsk : rattachées à fac_lithuania (Vitebsk : Iaroslav nominal, Algirdas hérite en 1345).
  Brest/Podlachie (Drohiczyn) : contesté avec la Mazovie et la Volhynie.
- Valachie : Craiova (1475), Brăila (1368), Buzău (1431) attestées tard ; localisations « centre régional ».
- Héraldique : Lituanie = trois pals (colonnes de Gediminas approximées, bordure d'or de jeu) ; Płock/Varsovie/Rawa :
  bordures et lambel de jeu ; Kiev croix d'or (trident non dessinable) ; Transylvanie, Slavonie, Głogów : substitutions.
  Le rendu ne dessine pas les couronnes ni les croissants (aigles de Pologne et de Płock différenciés par la bordure).
- Croatie intérieure : Modruš/Frankopans et le littoral relèvent de D4 (pas de doublon vu côté D2 : seules Knin, Bihać, Modruš).
- Portraits : nouvelles factions ajoutées au bucket « italy_empire » (pas d'aire dédiée, pas de génération d'images).
- Fichiers partagés modifiés en ajout seulement : `data/ui/front_end.json`, `data/heraldry/houses.json`,
  `data/portraits/archetypes.json` (bucket italy_empire), `data/rules/mercenaries.json` (`region_names` : 10 régions).
- Fichiers existants modifiés : `fac_bohemia`/`fac_austria` (relations), `tit_bohemia` (+ moravia, wroclaw),
  `tit_austria` (+ styria, carinthia, carniola), `prov_austria` + son fichier de colonies.
- `cul_ruthenian` : fichier de noms fourni par D3 (`names_ru.json`), volontairement absent ici.
- Relations vers des factions d'autres lots non écrites (Ordre teutonique D1, Horde D3, Serbie/Bosnie D4) : à ajouter à la fusion.

## Tests rouges attendus (géo non régénérée, brief)
- `test_settlements_schema.py::test_settlement_file_is_valid[prov_*]` (28 de mes provinces : hors de l'ancienne grille 4096).
- `test_horizon.py::test_every_province_has_a_tile`, `test_settlement_graph.py` (3 erreurs de fixture) : artefacts géo.
- Rust : compteurs figés (`new_1337_matches_game_data` etc.) à réviser après régénération.
