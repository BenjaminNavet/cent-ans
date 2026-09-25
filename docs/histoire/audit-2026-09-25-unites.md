# Audit historique — unités, navires, technologies (25 septembre 2026, lot B5)

Relecture des 27 types d'unités (`data/unit_types/`), des 4 navires (`data/naval/ships/`) et des
45 technologies (`data/technologies/`), faite en rédigeant leurs fiches du Codex (lot B5 de
`docs/design/2026-09-25-bulles-partout.md`). Classement repris de l'audit H1 (`audit-2026-09-23.md`) :

- **Erreur** : fait faux, corrigé dans le fichier.
- **Approximation assumée** : fait discutable, laissé tel quel et signalé.
- **Anachronisme de jeu** : écart connu, conservé pour la jouabilité.

Seuls les textes, notes et sources ont changé ; aucune mécanique (coûts, statistiques, cultures,
prérequis) n'a été touchée.

## Bilan

| Domaine | Erreurs corrigées | Autres modifications |
|---|---|---|
| Technologies | 25 (22 listes de sources copiées-collées, 3 notes d'année) | 17 descriptions enrichies de liens |
| Unités | 3 | 27 descriptions avec liens `[[cdx_…]]` |
| Navires | 2 | 3 descriptions avec liens |

## 1. Technologies

### Erreurs
- **Sources génériques (22 fichiers).** `tech_artillery_fortification`, `tech_bombards`,
  `tech_bookkeeping`, `tech_coat_of_plates`, `tech_crossbow_windlass`, `tech_dismounted_tactics`,
  `tech_field_artillery`, `tech_full_plate`, `tech_gunpowder`, `tech_hospital_reform`,
  `tech_longbow_drill`, `tech_masonry`, `tech_paper_mills`, `tech_printing_press`,
  `tech_royal_taxation`, `tech_siege_engineering`, `tech_standing_companies`,
  `tech_three_field_rotation`, `tech_universities`, `tech_urban_sanitation`, `tech_water_mills`
  et `tech_windmills` citaient tous la même liste : « Guerre de Cent Ans, Armure de plates,
  Artillerie médiévale, Économie médiévale ». Cette liste, sans rapport avec le sujet (les moulins
  renvoyaient à l'armure de plates), est remplacée par les articles Wikipédia pertinents.
- **`tech_field_artillery`, note.** « Couleuvrines des frères Bureau à Formigny (1450) » : à Formigny,
  les deux couleuvrines décisives sont amenées par Louis Giribault ; Jean Bureau commande le parc
  retranché de Castillon (1453). Note corrigée.
- **`tech_bombards`, note.** « Bombardes anglaises à Crécy (1346) » présentait comme sûr un fait
  discuté. La note cite désormais le manuscrit de Walter de Milemete (1326), des canons discutés à
  Crécy et attestés au siège de Calais (1346-1347). Ajout de `uncertain: true`.
- **`tech_longbow_drill`, note.** « assize of arms, statuts d'Édouard III » était vague et en partie
  inexact (l'assise des armes date de 1181). Remplacé par le statut de Winchester (1285) et l'ordre
  d'Édouard III de 1363 qui impose le tir le dimanche.

### Précisions
- **`tech_gothic_flamboyant`, note.** « Chapelle Notre-Dame-de-la-Grange d'Amiens (vers 1375-1380) »
  devient « remplages de la chapelle de la Grange, cathédrale d'Amiens (v. 1373-1375) ».

### Approximations assumées
- `tech_compagnies_d_ordonnance` : « quinze compagnies de cent lances ». Le texte de l'ordonnance
  de 1445 est perdu ; ce chiffre, traditionnel, vient des chroniqueurs.
- `tech_handgonnes` : l'année 1364 (Pérouse) est retenue pour les armes à feu portatives. C'est une
  date plausible, pas une invention précise.
- `tech_aqua_vitae` est désormais lié à la fiche `cdx_eau_de_vie`, et non plus à `cdx_romarin` :
  le lien avec l'« eau de la reine de Hongrie » est une légende (voir l'anachronisme de la fiche du
  romarin).

## 2. Unités

### Erreurs
- **`unit_culveriners`.** « Audenarde 1382 » : aucune troupe de couleuvriniers n'est attestée au siège
  d'Audenarde, où Gand emploie surtout une grosse bombarde. Le mot « couleuvrine » n'apparaît en
  France qu'au début du XVe siècle. La mention est retirée ; restent Formigny et Castillon.
  L'anachronisme (unité recrutable dès 1380) est signalé dans la fiche `cdx_couleuvriniers`.
- **`unit_coutiliers`.** « tiennent les chevaux » : dans la lance fournie, ce rôle revient au page ou
  au valet. Le coutilier combat aux côtés de l'homme d'armes. Description corrigée.
- **`unit_flemish_pikemen`.** « Milices des métiers de Gand, Bruges et Ypres ; ont brisé la chevalerie
  française à Courtrai » : à Courtrai (1302), Gand, restée largement fidèle au roi, n'envoie qu'un petit
  contingent ; la victoire est surtout celle de Bruges, du Franc et d'Ypres. Villes réordonnées, et
  Roosebeke (1382) ajoutée.

### Approximations assumées
- `unit_flemish_pikemen` : le nom local « goedendag-dragers » (porteurs de goedendag) convient mal à
  des piquiers ; il est gardé, car la milice mêlait piques et goedendags au premier rang.
- `unit_ecorcheurs` : la culture `cul_german` parmi les cultures de recrutement est discutable (les
  écorcheurs ravagent l'Alsace mais sont surtout français, gascons et castillans). C'est une règle de
  jeu, laissée telle quelle.
- `unit_breton_knights` : le nom local « marchegerien » est une forme bretonne moderne
  (marc'hegerien) plutôt que médiévale.
- `unit_genoese_crossbowmen` : l'équipement « arbalète à étrier » est plausible. Les Génois de Crécy
  bandaient aussi au crochet de ceinture.
- `unit_mangonel` : le mot « mangonneau » est flou dans les sources. Le jeu en fait un engin léger,
  ce qui est cohérent avec l'usage du XIVe siècle.

### Anachronismes de jeu
- `unit_culveriners`, disponibles dès 1380 : voir ci-dessus.
- `unit_coutiliers` recrutables par la Bretagne et la Bourgogne dès 1445. Les ordonnances
  bourguignonnes qui les organisent datent de 1471-1473.

## 3. Navires

### Erreurs
- **`ship_nef`.** « Grand navire rond à deux mâts » : dans les mers du Nord du XIVe siècle, les grands
  navires restent à un mât. Le second mât ne se répand qu'à la fin du siècle et au XVe (en
  Méditerranée, plus tôt). Description corrigée : « les plus grands reçoivent un second mât à la fin
  du XIVe siècle ».
- **`ship_nef`, source.** « Christophe (cogue royale anglaise) » citait une cogue comme source de la
  nef. Remplacée par « Grace Dieu (1418) », la grande nef d'Henri V.

### Approximations assumées
- `ship_galley` : affichée « Galère », terme de l'époque moderne ; au XIVe siècle on dit « galée »
  (clos des Galées). La fiche s'intitule « La galée » et reconnaît les deux formes.
- `ship_galley`, champ `ram` : la galée médiévale porte un éperon au-dessus de la ligne de flottaison,
  qui brise les rames et sert de pont d'abordage, pas un rostre d'éperonnage antique. Le jeu lui donne
  des dégâts d'éperonnage modestes.

## Fiches du Codex

Chaque entité a maintenant une fiche liée par `entity`. Voir `docs/wip/b5-unites.md` pour la liste et
les réaffectations (`cdx_compagnies_ordonnance` → unité des gendarmes, `cdx_franc_archer` → unité des
francs-archers ; nouvelles fiches `cdx_ordonnance_louppy` et `cdx_ordonnance_montils` pour les
technologies correspondantes).
