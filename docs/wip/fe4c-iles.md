# FE4c — Registre féodal des îles Britanniques (1337)

Branche `feat/fe4c-iles`, issue de `main` (a2672d2e). Recette : F4a (voir
`docs/wip/fe4a-france.md`, `docs/wip/fe4a-assets.md`).

## Périmètre retenu (justification)

9 nouvelles factions jouables, 11 nouvelles provinces. Cible de l'orchestrateur ≈10/≈12 ;
préféré un lot plus petit mais entièrement sourcé à un remplissage approximatif.

### Factions retenues
- **Irlande anglo-normande** (vassales de jure de `tit_ireland`, elle-même vassale de
  `tit_england`) : `fac_kildare` (FitzGerald, comte de Kildare), `fac_ormond` (Butler, comte
  d'Ormond), `fac_desmond` (FitzGerald, comte de Desmond).
- **Irlande gaélique** (royaumes souverains *de jure*, jamais entrés dans la féodalité anglo-
  normande — pas de suzerain dans les données) : `fac_tyrone` (Ó Néill, roi de Tír Eoghain),
  `fac_thomond` (Ó Briain, roi de Thomond), `fac_connacht` (Ó Conchobhair, roi de Connacht),
  `fac_leinster` (Mac Murchada, roi de Leinster — faction fragile, `uncertain: true` sur
  plusieurs points).
- **Écosse** : `fac_isles` (MacDonald, seigneurie des Îles, vassale *de jure* de `tit_scotland`).
- **Angleterre** : `fac_lancaster` (comte de Lancastre, vassal de `tit_england`) — promotion en
  faction jouable d'un fief déjà personnellement tenu dans les données existantes
  (`tit_lancashire`, holder `chr_henry_of_grosmont`), corrigé au passage (voir plus bas).

### Écartés (choix assumés, à discuter si désaccord)
- **Principauté de Galles** : titre `tit_wales` créé (duché, vassal de `tit_england`,
  `holder_1337: fac_england`), **pas de faction** — vacante en 1337 (dernier titulaire fut le
  futur Édouard III ; Édouard de Woodstock ne devient prince de Galles qu'en 1343). Conforme à
  la consigne « titre, pas forcément faction ».
- **Comté de Chester** : laissé tel quel (domaine propre de la couronne via `prov_lancashire`
  existant, tenu pour le prince Édouard) — pas de nouvelle faction, pas de nouvelle province.
- **Durham (palatinat épiscopal)** : écarté. Modéliser un évêque temporel (succession élective,
  gouvernement théocratique) pour un seul comté dépasse le budget de ce lot ; laissé en note
  pour un lot futur si souhaité.
- **Marche de Galles (Mortimer)** : écarté — comté de March en abeyance en 1337 (Roger Mortimer
  exécuté 1330, petit-fils mineur), pas de titulaire net à cette date.
- **Île de Man** : `prov_man` créée (nouvelle province) mais **pas de faction** — tenue par
  Guillaume de Montaigu (comte de Salisbury), personnage anglais, pas un vassal irlandais/
  écossais distinct ; `owner: fac_england`.
- **Grands comtes écossais** : aucun ajouté — l'Écosse (David II/régence) n'est pas dans mon
  périmètre (déjà jouable), et une nouvelle scission interne (ex. Edward Balliol) est déjà
  modélisée par F0 comme fiefs personnels dans `fac_england` (`tit_galloway`, `tit_lothian`,
  holder `chr_edward_balliol`) — non touché.

## Provinces nouvelles (11)

Irlande : `prov_kildare`, `prov_meath` (subdivisées de `prov_dublin`), `prov_ormond`,
`prov_thomond`, `prov_desmond` (subdivisées de `prov_munster`), `prov_connacht` (nouvelle zone,
non couverte jusqu'ici), `prov_tyrone` (subdivisée de `prov_ulster`), `prov_leinster`
(subdivisée de `prov_dublin`, Wicklow/Carlow), `prov_man` (nouvelle, île de Man).
Angleterre : `prov_lancaster` (subdivisée de `prov_lancashire`).
Écosse : `prov_isles` (subdivisée de `prov_highlands`, Hébrides/Argyll).

## Titres

Nouveaux : `tit_ireland` (duché, vassal `tit_england`, holder `fac_england`), `tit_wales`
(duché, vassal `tit_england`, holder `fac_england`), `tit_kildare`, `tit_ormond`, `tit_desmond`
(comtés, vassaux de `tit_ireland`), `tit_tyrone`, `tit_thomond`, `tit_connacht`, `tit_leinster`
(royaumes souverains, pas de liege), `tit_isles` (duché, vassal `tit_scotland`), `tit_lancaster`
(comté, vassal `tit_england`).

`tit_england.de_jure_provinces` : retrait de `prov_dublin`, `prov_munster`, `prov_ulster`,
`prov_deheubarth`, `prov_gwynedd` (déplacées sous `tit_ireland`/`tit_wales`).

## Bogue corrigé au passage

`tit_lancashire` (préexistant) donnait `chr_henry_of_grosmont` comme titulaire 1337 du comté de
Lancastre — faux : en 1337 le titulaire est son père Henri, 3e comte de Lancastre (aveugle
depuis ~1330, mort en 1345) ; Henri de Grosmont n'est fait comte de Derby que le 16/03/1337 et
ne devient comte de Lancastre qu'en 1345. Créé `chr_henry_iii_de_lancastre` (ruler de
`fac_lancaster`) ; `chr_henry_of_grosmont` passe de `fac_england`/`commander` à
`fac_lancaster`/`heir` (ses propres titres, dont Derby 1337, restent inchangés). Effet de bord
probable sur les fixtures de décompte (armées/personnages de fac_england) — à vérifier aux
tests, comme pour l'aîné Valois de F4a.

## État

En cours d'écriture des fichiers JSON (titres, factions, provinces, personnages, colonies).

## Prochaine étape

Écrire tous les fichiers, valider schémas, régénérer le pipeline géo, lancer les tests.
