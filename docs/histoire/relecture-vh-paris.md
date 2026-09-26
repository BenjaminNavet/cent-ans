# Relecture historique : Paris vers 1340 (VH5, ADR 0078)

Relecture des faits datés de `data/landmarks_v2/paris.json`, écrit par
`tools/geo/paris_v2_author.py` (corrections faites dans le script, puis script relancé et
`cent-ans geo landmarks --city paris` : le fichier est régénéré à l'identique hors corrections).
Seuls des faits ont été tirés des sources ; rien n'est extrait des sources à licence non
commerciale. Verdicts : **confirmé** (source sérieuse d'accord), **corrigé** (le fichier a
changé), **incertain** (aucune source ne tranche ; la restitution reste marquée comme telle).

## Sources principales

| Réf. | Source |
|---|---|
| Brut 2017 | Catherine Brut, « Le pont parisien de Charles le Chauve », *Bulletin monumental*, 175-2, 2017 — https://www.persee.fr/doc/bulmo_0007-473x_2017_num_175_2_13052 |
| Van Ossel 1992 | Paul Van Ossel, « Nouvelles données sur l'enceinte de "Charles V" (XIVe-XVIe s.) à Paris, d'après les fouilles des jardins du Carrousel », *CRAI*, 136-2, 1992 — https://www.persee.fr/doc/crai_0065-0536_1992_num_136_2_15102 |
| Whiteley 1992 | Mary Whiteley, « Le Louvre de Charles V : dispositions et fonctions d'une résidence royale », *Revue de l'Art*, 97, 1992 — https://www.persee.fr/doc/rvart_0035-1326_1992_num_97_1_348001 |
| Hayot 2018 | Denis Hayot, *Paris en 1200*, CNRS éditions, 2018 (cité par Wikipédia, « Enceinte de Philippe Auguste » et « Petit Châtelet ») — https://fr.wikipedia.org/wiki/Enceinte_de_Philippe_Auguste |
| ALPAGE | P. Rouet, ALPAGE, *Paris en 1380*, couche des usages du sol (champs `TOPONYME`, `AUTRE_NOM`) — https://alpage.huma-num.fr/paris-in-1380/ |
| Bibale | IRHT-CNRS, Bibale, notice « Abbaye Saint-Victor (Paris, 1113-1790) » — https://bibale.irht.cnrs.fr/1424 |
| NDP | Cathédrale Notre-Dame de Paris, « Les architectes de Notre-Dame » — https://www.notredamedeparis.fr/comprendre/architecture/les-architectes-de-notre-dame/ |
| Flèche | Wikipédia, « Flèche de Notre-Dame de Paris » (D. Sandron ; J. du Breul, *Le théâtre des antiquités de Paris*, 1612) — https://fr.wikipedia.org/wiki/Fl%C3%A8che_de_Notre-Dame_de_Paris ; https://en.wikipedia.org/wiki/Spire_of_Notre-Dame_de_Paris |
| AFGC | Association française de génie civil, fiches « Petit Pont », « Pont au Change », « Pont de la Tournelle » — https://www.afgc.asso.fr/history-heritage/pont-de-la-tournelle-a-paris/ |

## Tableau des faits

| Fait (fichier avant relecture) | Verdict | Source | Changement |
|---|---|---|---|
| Grand-Pont en 1340 : bois, 10 m de large, maisons ; « 106 × 27 m » cités pour 1413 | **Corrigé** (précisé) / largeur **incertaine** | Brut 2017 ; AFGC « Pont au Change » ; Wikipédia « Pont Notre-Dame » (dimensions de 1413) | Le Grand-Pont s'effondre le 20 décembre 1296 ; en 1340 c'est le pont aux Changeurs (au roi), sur le tracé ALPAGE. Bois (il brûle en 1621). Largeur de 1340 inconnue : 10 m gardés comme restitution, `certainty: probable` ajouté, note sourcée |
| Pont aux Meuniers en 1340 (`probable`) | **Confirmé** | Brut 2017 | Existe depuis 1296 en aval, passerelle des moulins du chapitre de Notre-Dame ; nombre de moulins restitué (jusqu'à 13 plus tard). Note sourcée ; `probable` gardé (gabarit) |
| Planches de Mibray en 1340 | **Confirmé** (existence) / gabarit **incertain** | Brut 2017 (axe antique, « planches de Milbray » puis pont Notre-Dame) ; Wikipédia « Rue de la Planche-Mibray » (F. et L. Lazare : « les Planches-de-Mibray » en 1313) ; Wikipédia « Pont Notre-Dame » (tenues jusqu'aux crues de 1406) | Note réécrite (planches posées sur les anciennes piles), `certainty: probable` ajouté |
| Petit-Pont : pierre de 1186, ≈ 40 m, maisons | **Confirmé**, avec réserve | AFGC « Petit Pont » ; Hayot 2018 via Wikipédia « Petit Châtelet » (crue du 20 décembre 1296) | Note : renversé en 1296 puis rebâti, état exact de 1340 non documenté |
| Petit Châtelet « vers 1130, reconstruit après 1296 », `attested`, 13 m | **Corrigé** | Hayot 2018 (rebâti par Philippe Auguste en 1205-1212, murs de 1,95 m, 15,68 m de haut) ; Wikipédia « Petit Châtelet » (renversé en 1296, reconstruit en 1369 par Hugues Aubriot) | Hauteur 13 → 16 m (gabarit de 1212), `certainty` → `probable` (état de 1340 mal connu), description sourcée |
| Flèche de Notre-Dame ≈ 78 m (croisée 45 m + flèche 33 m), « posée vers 1250 » | **Confirmé** (hauteur) / datation **incertaine** | Flèche : du Breul (1612) donne 78 m du sol à la pointe ; D. Sandron 83 m ; datation vers 1220-1230, vers 1250 (Sandron) ou années 1290 (dendrochronologie) | Gabarit inchangé (78 m) ; description donne les deux hauteurs et les trois datations (toutes antérieures à 1340) |
| Notre-Dame : « chapelles et derniers travaux vers 1345 » | **Corrigé** (précisé) | NDP : Jean Ravy maître d'œuvre 1318-1344 (chapelles du chevet, arcs-boutants du chœur, clôture du chœur), Jean le Bouteiller achève la clôture en 1351 | Description : chantier Ravy en 1340 ; gabarit inchangé (l'édifice est complet vu de l'extérieur) |
| Enceinte de Charles V : levée 1356-1364, maçonnée à partir de 1365 | **Confirmé** | Van Ossel 1992 (travaux de 1356 sous Étienne Marcel, nature de l'ouvrage mal connue ; reprise par Charles V à partir de 1365, achèvement vers 1420) | Notes sourcées ; dates inchangées ; gabarit de la levée toujours hypothétique |
| Louvre de Charles V à partir de 1364 | **Confirmé** | Whiteley 1992 ; Wikipédia « Raymond du Temple » (maître des œuvres le 22 avril 1364, grande vis à partir de 1364-1365) | Description sourcée ; `until_year` 1363 / `from_year` 1364 inchangés |
| Donjon du Temple « avant 1310 », `probable` | **Confirmé** (présent en 1340) / date exacte **incertaine** | Wikipédia « Tour du Temple » (vers 1240, ≈ 50 m, carré de 15 m, murs de 4 m) ; notice « Le donjon du Temple de Paris » (Hubert † 1222 / Jean de Tour † v. 1310 ; 1265-1270 selon L. Delisle) | Description : ≈ 15 m de côté, ≈ 50 m, fourchette de datation ; gabarit (50 m) inchangé |
| Saint-Victor fondée en 1113 « selon la tradition, à vérifier », `probable` | **Confirmé** | Bibale (IRHT) « Abbaye Saint-Victor (Paris, 1113-1790) » ; Guillaume de Champeaux s'y retire en 1108, Louis VI l'érige en abbaye en 1113 | `certainty` → `attested`, description sourcée |
| Bernardins : église de 1338 inachevée, `hypothetical` | **Confirmé** / gabarit **incertain** | Wikipédia « Collège des Bernardins » (première pierre le 24 mai 1338, Benoît XII ; jamais achevée) | Description précisée ; le modèle reste une église complète marquée `hypothetical` (en 1340 le chantier débute : voir points ouverts) |
| Porte « Saint-Honoré (dite aux Aveugles) » | **Confirmé** | ALPAGE : « Porte aux Aveugles », autre nom « Ancienne Porte Saint-Honoré » ; Wikipédia (Hayot 2018) | Aucun (le nom de 1340 est bien porte Saint-Honoré) |
| Porte « du Temple (porte de Braque) » | **Corrigé** | ALPAGE : « Porte du Temple » sans autre nom ; Wikipédia « Rue de Braque » (Arnoul de Braque fonde sa chapelle en 1348 près de la porte du Chaume) | Renommée « Porte du Temple (Sainte-Avoie) » ; note : « porte de Braque » désigne la poterne du Chaume, et pas avant 1348 |
| Porte « Baudoyer (Saint-Antoine) » | **Corrigé** (ordre des noms) | ALPAGE : « Porte de Saint-Antoine » ; Wikipédia (Gagneux et Prouvost 2004) : porte Saint-Antoine, dite Baudet ou Baudoyer, démolie en 1382 | Renommée « Porte Saint-Antoine (dite Baudet ou Baudoyer) » ; démolition de 1382 notée |
| Porte « de Buci » (rive gauche) | **Corrigé** | Wikipédia (Hayot 2018) : la porte Saint-Germain prend le nom de Buci en 1352 (bail de Simon de Buci, 1350) | Renommée « Porte Saint-Germain (porte de Buci en 1352) » |
| Porte « Saint-Germain (des Cordeliers) » | **Corrigé** | Idem : porte des Cordeliers, percée en 1240, renommée Saint-Germain plus tard | Renommée « Porte des Cordeliers » |
| Porte « Saint-Michel (d'Enfer) » | **Corrigé** | Wikipédia « Porte d'Enfer » et Hayot 2018 : porte Gibard puis d'Enfer, nommée Saint-Michel en 1394-1395 | Renommée « Porte d'Enfer ou Gibard (porte Saint-Michel en 1394) » |
| Porte « de la Comtesse d'Artois » | **Incertain** | ALPAGE (1380) : « Porte Comtesse d'Artois » ; Wikipédia : « porte Nicolas Arode » sur le plan de Guillot (restitution du XIXe s.) | Inchangé |
| Fossés de Philippe Auguste absents en 1340 | **Confirmé** | Hayot 2018 via Wikipédia (enceinte « dépourvue de fossés à l'origine ») ; fossés du XIVe s. (Van Ossel 1992 ; fouille Inrap de l'Institut, fossé de Charles V) | `ditch_m: 0` inchangé, note sourcée |
| Tour de l'Horloge à partir de 1350 | **Confirmé** | Wikipédia « Tour de l'Horloge » (1350-1353, horloge de 1371) ; `docs/research/vh-sources.md` | Aucun |
| Bastille à partir de 1370 | **Confirmé** | France Pittoresque, « 22 avril 1370 : pose de la première pierre de la Bastille » | Description : première pierre le 22 avril 1370, achèvement vers 1383 |
| Pont Saint-Michel à partir de 1378 | **Confirmé** | Wikipédia « Pont Saint-Michel (Paris) » (décidé en 1353, construit 1378-1379 à 1387 par Hugues Aubriot) | Note précisée ; `from_year` 1378 gardé (début des travaux, test inchangé) |
| « Pont de la Tournelle » à partir de 1370 | **Confirmé**, nom **corrigé** | AFGC « Pont de la Tournelle » (pont de Fust de l'île Notre-Dame, 1369-1370) | Renommé « Pont de Fust de l'île Notre-Dame (futur pont de la Tournelle) » |
| « Sainte-Agnès » (fid 8, `probable`) | **Corrigé** | Wikipédia « Église Saint-Eustache » : chapelle Sainte-Agnès érigée en paroisse Saint-Eustache en 1223 ; position ALPAGE entre les portes Coquillière et Montmartre | Renommée « Saint-Eustache (ancienne chapelle Sainte-Agnès) », `attested` (identifiant `sainte_agnes` gardé) |
| Chapelle des Filles-Dieu (fid 403) présente en 1340 | **Corrigé** | Atlas historique de Paris, « Le couvent des Filles-Dieu » (http://paris-atlas-historique.fr/resources/Couvent+des+Filles-Dieu.pdf) ; BnF, data.bnf.fr « Filles-Dieu. Paris (1226-1798) » : fondées en 1226 hors les murs, repliées en 1360 rue Saint-Denis dans l'enceinte de Charles V | `from_year: 1360` (le site ALPAGE est celui d'après 1360) ; le premier site n'est pas restitué |
| Autres monuments `probable` (palais, halles, couvents, églises à reconstruction tardive) | **Confirmé** (existence en 1340) | Noms comparés un à un aux toponymes ALPAGE 1380 ; dates de fondation déjà sourcées (Célestins 1352/1367, Saint-Yves 1348-1357, Saint-Esprit 1362, Petit-Saint-Antoine 1361, Maison aux Piliers 1357, Saint-Sépulcre 1325-1326) | Aucun : le caractère `probable` porte sur le gabarit |
| Quartiers et faubourgs tirés des zones bâties de 1380 | **Incertain** | Aucune carte de 1340 à cette échelle | Inchangé (écart de 40 ans assumé) |

## Points ouverts

- **Bernardins** : en 1340 le chantier de 1338 débute ; une église complète (gabarit ALPAGE de 1380)
  reste affichée, marquée `hypothetical`. Faute de source sur l'état de 1340, pas de gabarit
  « chantier » inventé.
- **Porte Saint-Antoine de Philippe Auguste** démolie en 1382 : le moteur accepte des dates sur
  les portes, mais retirer une porte laisserait un trou dans la muraille ; non daté.
- **Premier site des Filles-Dieu** (1226-1360, faubourg Saint-Denis) : absent d'ALPAGE, non
  restitué.
- **Largeur du pont aux Changeurs** en 1340 et **nombre de moulins** du pont aux Meuniers : aucune
  source ; restitutions gardées.
- **Petit Châtelet et Petit-Pont** entre 1296 et 1369 : état réel inconnu (ruines réparées ?).
- `data/landmarks/paris.json` (format v1) n'a pas été relu (hors mission).
