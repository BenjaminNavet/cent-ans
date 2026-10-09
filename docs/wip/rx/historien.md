# RX — Historien (médiéviste, 1337-1453)

Périmètre audité par script : 253 personnages (dates, filiations, titres), 177 factions (souverains, capitales, blasons, relations de départ), 443 provinces et 2147 établissements (noms, propriétaires, bâtiments), 153 événements (dates, saisons), 540 fiches de Codex (échantillon d'une centaine lu), 45 unités/technologies. Les restes déjà connus de `docs/histoire/audit-2026-09-30.md` ne sont pas répétés.

## Verdict

- Forces : l'exactitude factuelle est très haute. Sur ~250 personnages, les dates de naissance et de mort de tous les souverains et de la plupart des seconds rôles correspondent à l'historiographie (un seul écart net, Philippa de Hainaut). Les 153 événements portent des dates de jour exactes (Cadzand, Arnemuiden, Southampton, Morlaix, Poitiers, Brétigny, Jacquerie, Tain-l'Hermitage, Louppy 1445, Montils 1448...). Les fiches de Codex vérifient bien leurs propres anachronismes (champ `anachronism`, descriptions d'établissements qui datent leurs fondations).
- Les unités sont bien bornées par `available_from` (ordonnance 1445, francs-archers 1448, routiers 1356, couleuvriniers 1380).
- Faiblesses : quelques oublis de données au départ (aucune université à Paris), un anachronisme de nom (Novi Pazar), un anachronisme de titre dans un texte d'événement (« duc d'Alençon »), trois blasons douteux, des incohérences internes entre fiche et unité (cranequin), et une chronique 1337-1345 mince pour l'Occident (Majorque, Épire, Gênes, Galicie, retour de David II sans événement).
- Aucun constat bloquant. Les écarts sont des corrections de données (S) ou des événements à ajouter (M).

## Constats (du plus grave au moins grave)

### [majeur] Aucune université à Paris, Toulouse, Orléans, Cambridge — type bug (données)
**Constat** : la première université d'Occident n'a pas de bâtiment `bld_university`, alors que la description de la province la cite et que la France part avec `tech_universities`. Le joueur France ne peut pas y exploiter le Studium. Seules 12 villes portent le bâtiment (Salamanque, Avignon, Lisbonne, Fès, Naples, Oxford, Lérida, Montpellier, Bologne, Florence, Tunis, Padoue) ; manquent aussi Toulouse (1229), Orléans (légale vers 1306), Cambridge (vers 1209), Angers, Pérouse (1308), Sienne, Valladolid, Le Caire (al-Azhar est présent sous Cairo en province seulement).
**Preuve** : `data/provinces/prov_ile_de_france.json` (buildings : windmill, vineyard_press, cathedral, fair, hotel_dieu, castle, muster_field, stables) ; `data/settlements/prov_ile_de_france.json` entrée `set_paris` (même liste, pas d'université, pas de `bld_stone_walls` alors que fortification_level 3) ; recensement `grep bld_university` = 12 établissements. `cdx_universite_paris` existe.
**Correction proposée** : ajouter `bld_university` à `set_paris` (et `prov_ile_de_france`), `set_toulouse`, `set_orleans`, `set_cambridge` (s'il existe, sinon `prov_norfolk`/Est-Anglie), `set_perugia`, `set_angers` ; ajouter `bld_stone_walls` à Paris. Vérifier l'équilibre (coût d'entretien) sur la France jouable.
**Coût** : S.

### [mineur] « Duc d'Alençon » à Crécy — type finition (anachronisme de titre)
**Constat** : le texte de l'événement parle du « duc d'Alençon ». Charles II est comte d'Alençon (le duché date de 1414). Les données personnage, faction et Codex disent « comte ».
**Preuve** : `data/events/evt_crecy.json` ligne 4 ; `data/characters/chr_charles_d_alencon.json` (« Comte d'Alençon »).
**Correction proposée** : remplacer par « le comte d'Alençon, frère du roi ».
**Coût** : S.

### [mineur] Novi Pazar, nom postérieur à 1461 — type bug (nom de lieu)
**Constat** : « Novi Pazar » est fondée par Isa-Beg Ishaković en 1461 (nom turco-slave). En 1337 le marché de l'Ibar en Rascie est Trgovište (Pazar), voisin du site de Ras.
**Preuve** : `data/settlements/prov_raska.json` entrée `set_novi_pazar` (display « Novi Pazar », description « Marché de l'Ibar »).
**Correction proposée** : display « Trgovište (Ras) », local « Трговиште ». Seul cas trouvé par balayage des noms en Novi/Nova/Neu/Nowy et des toponymes modernes (Tromsø et Umeå sont des paroisses attestées, acceptables).
**Coût** : S.

### [mineur] Philippa de Hainaut née en 1310 au lieu de 1314 — type bug (date)
**Constat** : la reine naît le 24 juin 1314 (mariage à 13-14 ans en janvier 1328, premier enfant en 1330). La donnée la fait naître quatre ans trop tôt, donc 27 ans en 1337 au lieu de 22-23 : effet direct sur la fécondité simulée (enfants nés après 1337, Lionel, Jean de Gand, etc.).
**Preuve** : `data/characters/chr_philippa_of_hainault.json` : `"birth": "1310-06-24"`.
**Correction proposée** : `1314-06-24`. (Même type de vérification utile pour Théodora Basarab née 1310 mère de Michel Asen IV né 1322 : écart d'âge de 12 ans, `chr_theodora_basarab.json`, à ramener vers 1308.)
**Coût** : S.

### [mineur] Gascon : arbalète « à cranequin » en 1337 — type finition (incohérence interne)
**Constat** : l'équipement des arbalétriers gascons cite le cranequin, alors que les fiches `cdx_arbalete` et `cdx_arbalete_a_tour` (et la technologie `crossbow_windlass`) datent le cranequin de la fin du XIVe siècle.
**Preuve** : `data/unit_types/unit_gascon_crossbowmen.json` (equipment : « étrier ou à cranequin ») ; `data/codex/cdx_arbalete_a_tour.json` (« à partir de la fin du siècle »).
**Correction proposée** : « à étrier ou à tour » (ou « à crochet »).
**Coût** : S.

### [mineur] Trois blasons à vérifier — type finition (héraldique)
**Constat** :
- Montferrat : « D'argent à la fasce de gueules » ; les Aleramici/Paléologues portent « d'argent au chef de gueules » (cohérent avec Saluces voisin « au chef d'azur »).
- Sienne : « De sable à la fasce d'argent » ; la balzana est « coupé d'argent et de sable ».
- Navarre : l'émeraude au cœur des chaînes est une addition postérieure (XVe siècle) ; en 1337 les chaînes sont seules.
**Preuve** : `data/factions/fac_montferrat.json`, `fac_siena.json`, `fac_navarre.json` (`heraldry.blazon`). Confiance moyenne : à recouper avec un armorial (Gelre, Wijnbergen) avant correction ; le générateur d'écus sait déjà tracer le chef et le coupé.
**Correction proposée** : corriger les trois blasons, puis relancer le générateur d'écus pour ces factions.
**Coût** : S.

### [mineur] Écosse : sa capitale appartient à l'Angleterre au départ — type conception
**Constat** : `fac_scotland.capital = prov_lothian` (Édimbourg), province tenue par l'Angleterre au départ (historiquement, le château d'Édimbourg est anglais jusqu'en 1341, ce qui justifie la possession) ; le roi est aussi en exil en Normandie. L'Écosse est la seule faction de la partie de départ, avec les « Rebelles » et les Croisés, dont la capitale n'est pas à elle. Toute règle de « perte de capitale » ou d'affichage peut mal réagir. Perth (englobée dans `prov_fife`) est de plus anglaise jusqu'en août 1339, donc `prov_fife` écossaise est déjà une approximation.
**Preuve** : script factions/provinces (`capital not owned: fac_scotland prov_lothian owner fac_england`) ; `data/factions/fac_scotland.json`.
**Correction proposée** : documenter la liberté dans `capital_city` ou déplacer la capitale de départ vers `prov_fife` (Scone/Perth) tant qu'Édimbourg est anglaise, et prévoir l'événement du retour de David II (juin 1341) et de la reprise d'Édimbourg (avril 1341, ruse de Douglas).
**Coût** : S (capitale) / M (événements).

### [mineur] Chronique occidentale 1337-1345 mince — type conception (événements)
**Constat** : plusieurs faits datés, déjà présents dans les personnages ou les descriptions, n'ont aucun événement : annexion de l'Épire par Andronic III (automne 1337, Nicéphore II mineur sous la régence d'Anne Paléologine), révolution génoise de Simone Boccanegra (septembre 1339), mort de Boleslas-Iouri II et succession de Galicie-Volhynie (1340, Casimir III intervient), reconquête d'Édimbourg et retour de David II (1341), annexion de Majorque par Pierre IV (1343-1349), assassinat d'Artevelde (24 juillet 1345 : personnage mort, mais pas d'événement), siège d'Aiguillon (fiche Codex seule). Les mentions « Majorque », « Épire », « Boccanegra » apparaissent dans 0 événement.
**Preuve** : `grep -l` sur `data/events/` (0 résultat pour Majorque, Épire, Boccanegra, Galicie, Édimbourg, Perpignan) ; `chr_jacques_iii_de_majorque` (mort 1349-10-25), `chr_jacob_van_artevelde` (mort 1345-07-24), `chr_boleslas_iouri_ii` (mort 1340-04-07).
**Correction proposée** : 5 à 6 événements `historical` à `until_year` large, dans le format de `evt_prise_d_algesiras` / `evt_vente_de_l_estonie` (effets `transfer_title`, `set_ruler`). Priorité : Majorque et Artevelde (touchent la partie France/Angleterre/Aragon).
**Coût** : M.

### [mineur] Barrois possédé par la France, « Bar-le-Duc » avant 1354 — type finition (donnée, nom)
**Constat** : le comté de Bar (Henri IV de Bar, beau-frère de Philippe VI) n'a pas de faction ; `prov_bar` est à la France. La capitale s'appelle « Bar-le-Duc » alors que la fiche de Pierrefitte-sur-Aire rappelle que le comté ne devient duché qu'en 1354 : en 1337 on dit « Bar » ou « Bar-sur-Ornain ».
**Preuve** : `data/provinces/prov_bar.json` (owner fac_france, capital_city Bar-le-Duc) ; `data/settlements/prov_bar*.json` (`set_pierrefitte_sur_aire`, description « le comté ne devient duché qu'en 1354 »).
**Correction proposée** : renommer la capitale en « Bar (Bar-sur-Ornain) » ; envisager une faction `fac_bar` vassale de la France et de l'Empire (comme Lorraine) ou, à défaut, documenter l'approximation.
**Coût** : S (nom) / M (faction).

### [mineur] Bourbon : titre « comte de Clermont-en-Beauvaisis » en 1337 — type finition (titre)
**Constat** : `fac_bourbon.titles` garde « Comte de Clermont-en-Beauvaisis » en plus de « Comte de la Marche ». À ma connaissance, en 1327 Louis Ier a échangé Clermont contre la Marche lors de l'érection du duché ; Clermont revient à la couronne. Confiance moyenne : à contrôler avant de corriger.
**Preuve** : `data/factions/fac_bourbon.json` (`titles`) ; `chr_louis_i_de_bourbon.json` ne porte que Duc de Bourbon et Comte de la Marche.
**Correction proposée** : retirer Clermont de la liste si la vérification se confirme (le personnage porte déjà la bonne liste).
**Coût** : S.

### [mineur] « Le Prince noir » employé 38 fois sans signaler que le surnom est tardif — type finition (vocabulaire)
**Constat** : « Prince noir » n'est attesté qu'au XVIe siècle (Leland, Grafton), pas par les contemporains qui disent « le prince de Galles » ou « Édouard de Woodstock ». Le Codex et les événements l'emploient comme nom courant ; seule la fiche `cdx_prince_noir` devrait l'expliquer. Le jeu le dit lui-même pour la Jarretière (« la légende... n'apparaît qu'un siècle plus tard » : l'attestation est de Polydore Vergil, 1534, soit près de deux siècles).
**Preuve** : `grep -rl "Prince noir" data/` = 38 fichiers ; `data/codex/cdx_prince_noir.json` (aliases, aucune mention de la date du surnom) ; `data/events/evt_ordre_de_la_jarretiere.json`.
**Correction proposée** : une phrase de datation dans `cdx_prince_noir` ; dans l'événement Jarretière, « un siècle et demi plus tard » ou « au XVIe siècle ».
**Coût** : S.

### [mineur] Tyrol : le souverain est Jean-Henri, alors que la comtesse est Marguerite — type finition (cohérence)
**Constat** : `fac_tirol.ruler = chr_jean_henri_de_luxembourg`, Marguerite Maultasch est « consort ». En droit, Marguerite hérite en 1335 et Jean-Henri n'est que mari. Le Codex dit lui-même que les Luxembourg « gouvernent en son nom ». Le même cas est modélisé inversement pour la Navarre (Jeanne II souveraine, Philippe d'Évreux consort).
**Preuve** : `data/factions/fac_tirol.json`, `data/characters/chr_marguerite_de_tyrol.json` (role consort), `chr_jean_henri_de_luxembourg.json`.
**Correction proposée** : `ruler = chr_marguerite_de_tyrol`, Jean-Henri `regent`/`consort` jusqu'à son expulsion de 1341 ; attention à la loi de succession féminine de la faction.
**Coût** : S.

### [mineur] Noms de provinces ambigus — type finition
**Constat** : deux provinces portent « Cerdagne » (`prov_cerdagne` et « Roussillon et Cerdagne »), et la province anglaise « Ulster et Connacht » (Carrickfergus) coexiste avec `prov_connacht` (royaume gaélique de Connacht, Roscommon).
**Preuve** : `data/provinces/prov_cerdagne.json`, `prov_roussillon.json`, `prov_ulster.json`, `prov_connacht.json`.
**Correction proposée** : renommer « Comté d'Ulster (Carrickfergus) » et « Roussillon » / « Haute-Cerdagne (Puigcerdà) ».
**Coût** : S.

### [mineur] Technologies de départ postérieures à 1337 — type équilibrage / finition
**Constat** : le Maroc mérinide et Grenade démarrent avec `tech_urban_sanitation` (datée 1350, « après la Peste noire ») ; 11 factions hanséatiques démarrent avec `tech_hanseatic_trade` (datée 1356, première diète générale de Lübeck, alors que la Hanse existe dès le XIIIe siècle) ; `tech_royal_taxation` (1341, gabelle) est donnée à 20 factions. Les dates de la technologie sont celles d'un jalon, pas d'une origine.
**Preuve** : `data/technologies/tech_urban_sanitation.json`, `tech_hanseatic_trade.json` (`historical_year`) et `starting_technologies` des factions concernées.
**Correction proposée** : soit redater (`hanseatic_trade` 1260-1300, `royal_taxation` 1300), soit expliquer par une note « jalon » ; retirer `urban_sanitation` du départ des deux factions musulmanes (ou la requalifier hydraulique/hammam).
**Coût** : S.

### [mineur] Archery butts « sur ordre d'Édouard III (1363) » présents au départ — type finition
**Constat** : 16 provinces/établissements anglais démarrent avec `bld_archery_butts`, dont la description fait dater l'obligation de 1363 ; la pratique régulière date du statut de Winchester (1285) et des ordres d'Édouard Ier, mais pas la mesure citée.
**Preuve** : `data/buildings/bld_archery_butts.json` (description) ; 16 occurrences dans `data/provinces/` et `data/settlements/`.
**Correction proposée** : reformuler la description (« imposées à nouveau par Édouard III en 1363 ») ou conditionner le bâtiment par la technologie `longbow_drill`.
**Coût** : S.

## Ce qu'il ne faut surtout pas changer

- Les dates précises des événements et des personnages (le contrôle croisé de ~250 fiches, 150 événements, ~60 batailles citées dans le Codex n'a révélé aucune erreur d'année), le mécanisme `setup_1337` (naissances à venir, morts de 1337) et les champs `historical_date` / `until_year` souples.
- Le champ `anachronism` et les descriptions d'établissements qui datent leurs fondations (Vincennes, Chartreuse de Pavie, Kiel, Josselin...) : excellente pratique, à étendre aux nouvelles fiches.
- Le bornage des unités tardives par `available_from` et par `required_technology`, et le choix de ne pas donner d'archers longs aux Français avant les francs-archers.
- Les libertés déjà signalées par l'audit HV (Juliers, Charolais, alliance Empire-Angleterre dès le printemps 1337, Algésiras/Gibraltar).
- Les blasons attestés (Albret, Mark, Alençon, Armagnac, Blois, Bourbon, Lancastre, Pise, Calatrava) et la distinction France ancien / France moderne.
