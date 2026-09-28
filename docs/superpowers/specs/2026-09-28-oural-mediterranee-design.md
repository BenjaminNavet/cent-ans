# OM — Carte jusqu'à l'Oural et tout le pourtour méditerranéen

Demande du joueur (2026-09-28, soir) : « augmente la carte jusqu'à l'Oural et tout le pourtour
méditerranéen avec toutes les factions ». Suite annoncée de FE (`2026-09-28-feodalite-design.md`),
qui réutilise le modèle féodal (titres au-dessus des factions, 3 niveaux).

## Décisions

| Sujet | Décision |
|---|---|
| Projection | EPSG:3035 conservée ; 718,9765625 m par unité (ADR 0082 inchangé : pas de pas de marche ni de constante à recaler) |
| Emprise | Rectangle 7168 × 6144 unités (28 × 24 tuiles racines de 256), ADR 0115 |
| Origine | Bord ouest inchangé (x = 2 169 486 m) ; bas abaissé de 3 tuiles racines (y = 775 684 m) ; haut y = 5 193 076 m. Toute coordonnée pixel existante : x inchangé, **y + 1280** |
| Couverture | Maroc atlantique → Oural (Iekaterinbourg, Orenbourg) ; mer Blanche → delta du Nil et golfe de Syrte ; Levant, Anatolie, Caucase, Caspienne occidentale |
| Relief fin | La pyramide actuelle (tiers 1-3, Ouest) est gardée telle quelle par un décalage d'origine dans le manifeste ; pas de recuisson ni de nouveau paquet hébergé. L'Est et le Sud n'ont que le niveau de base (ETOPO 15″) en v1 |
| Textures de base | Monde entier à 719 m/texel (province_ids, splat, distances, relief ombré…), tuilées si besoin ; aucun fichier versionné > 50 Mo (limite GitHub 100 Mo) |
| Déserts et steppes vides | Terres à plus de ~400 km d'une graine laissées hors provinces (Sahara, Arabie, steppe kazakhe) : infranchissables, rendues en « terra incognita » |
| Factions | Toutes les puissances de 1337 des nouvelles régions, avec la même granularité féodale que FE à l'Ouest (≈ +110 factions, ≈ +230 provinces), provinces plus grandes dans la steppe et le désert |
| Terrains / climats | Nouveaux terrains `steppe`, `desert` ; nouveaux climats `arid`, `steppe` (ADR 0116) |
| Religions | `rel_orthodox`, `rel_armenian`, `rel_pagan` ; `rel_islam` devient l'islam sunnite général |
| Cultures | Identifiants libres, un fichier de noms par groupe linguistique nouveau |
| Mers | Nouvelles zones : `sea_ionian`, `sea_aegean`, `sea_levantine`, `sea_black_sea`, `sea_caspian` (fermée), `sea_gulf_of_bothnia`, `sea_norwegian`, `sea_white_sea`. `sea_mediterranean` = Méditerranée occidentale |
| Sauvegardes | Les sauvegardes antérieures ne se chargent plus (coordonnées et provinces changent) : message clair, pas de migration |
| Portraits | Souverains et héritiers des nouvelles factions, plafond propre 10 $ (après la géo et les données) |

## Régions et lots de données

| Lot | Région | Puissances principales |
|---|---|---|
| D1 | Nord et Baltique | Danemark (interrègne, gages holsteinois), Norvège-Suède (Magnus Eriksson) + Finlande, ordre Teutonique, ordre Livonien et évêchés (Riga, Dorpat, Ösel-Wiek, Courlande), Estonie danoise, Poméranie (Wolgast, Stettin) |
| D2 | Europe centrale et orientale | Pologne (Casimir III), Mazovie, duchés silésiens, Hongrie (Charles Robert : Croatie, Slavonie, Transylvanie), Galicie-Volhynie, Lituanie (Gediminas) et terres ruthènes, Valachie (Basarab), marches autrichiennes manquantes |
| D3 | Rus' et Horde d'Or | Novgorod (+ Grand Perm, Carélie), Pskov, Moscou (Ivan Kalita), Tver, Riazan, Souzdal-Nijni, Rostov, Iaroslavl, Smolensk, Briansk, principautés de Tchernigov ; Horde d'Or (Özbeg) : Saraï, Astrakhan, Bolgar, Mordves, Bachkirie, Crimée, Azaq, steppes ; Caffa génoise, Théodoro |
| D4 | Balkans, Byzance, Égée | Byzance (Andronic III), Serbie (Dušan), Bulgarie (Ivan Alexandre), Bosnie, Raguse, royaume angevin d'Albanie, duché d'Athènes, Achaïe, Archipel, colonies vénitiennes, Hospitaliers (Rhodes), Chypre |
| D5 | Anatolie, Caucase, Levant, Égypte | Ottomans (Orhan), Karasi, Saruhan, Aydın, Menteşe, Germiyan, Hamid, Teke, Karaman, Eretna, Candar, Trébizonde, Arménie cilicienne, Ramadanides, Dulkadir, Géorgie, Alains et Circassiens, sultanat mamelouk (Égypte, Palestine, Syrie), marges chobanides et djalayirides |
| D6 | Maghreb et compléments | Mérinides (Abu l-Hasan, Tlemcen prise en mai 1337), Hafsides (Tunis, Bougie, Constantine), Tripoli, Barqa, Djerba, Malte ; trous de la carte actuelle en bordure |

## Hors périmètre v1
Relief fin (tiers 1-3) à l'Est ; villes historiques 1:1 à l'Est ; unités propres à l'Est (archers
montés, mamelouks) — les armées réutilisent les types existants, un chantier ultérieur ajoutera les
unités ; événements historiques de l'Est au-delà des objectifs de faction.
