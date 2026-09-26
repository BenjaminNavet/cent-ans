# Relecture historique — Orléans 1:1 vers 1340-1429 (VH7)

Relecture du 26 septembre 2026 de `data/landmarks_v2/orleans.json` (ville 1:1 v2, ADR 0078 ;
le plan reste celui du siège de 1428-1429, ADR 0026). Méthode : recherche web dans des sources
sérieuses (Inrap, OpenEdition, Persée, service archéologique municipal, base universitaire des
collégiales), Wikipédia seulement en appoint quand elle cite ses sources (Chenesseau, Collin).
Seuls des faits sont repris ; rien n'est extrait des sources non commerciales (Gallica, dont
l'accès était de toute façon bloqué par une vérification anti-robot : Collin et le Journal du
siège ne sont connus ici que par les notices qui les citent).

Verdicts : **confirmé** (la donnée est conforme aux sources), **corrigé** (une source fiable la
contredit : le JSON a été modifié), **incertain** (sources divergentes ou muettes : donnée gardée,
note mise à jour).

## Sources principales

| Abr. | Référence |
|---|---|
| SAMO | Ville d'Orléans, service Ville d'art et d'histoire et service archéologique municipal, *Focus Orléans : les enceintes urbaines, circuit patrimonial*, 2014, rééd. 2023. https://www.orleans-metropole.fr/fileadmin/orleans/MEDIA/kiosque/ville_art_histoire/vdh_enceintes_urbaines.pdf |
| Carron 2012 | D. Carron et T. Guillemard, « La construction d'une enceinte à Orléans. La fortification des XIVe-XVe siècles », *Archéopages*, 33, 2011 [2012], p. 74-79. https://journals.openedition.org/archeopages/17524 |
| Petit 1987 | D. Petit, « Orléans (Loiret). Place du Martroi », *Revue archéologique du Centre de la France*, 26-1, 1987, p. 100. https://www.persee.fr/doc/racf_0220-6617_1987_num_26_1_2513_t1_0100_0000_2 |
| Inrap enceintes | Inrap, atlas archéologique d'Orléans, « Les enceintes d'Orléans depuis l'Antiquité ». https://multimedia.inrap.fr/atlas/orleans/syntheses/themes/enceintes-depuis-antiquite |
| Inrap De Gaulle | Inrap, atlas d'Orléans, « Deuxième ligne de tramway, place de Gaulle ». https://multimedia.inrap.fr/atlas/orleans/sites/2512/Deuxieme-ligne-de-tramway-place-de-Gaulle |
| Inrap Martroi | Inrap, atlas d'Orléans, « Place du Martroi ». https://multimedia.inrap.fr/atlas/orleans/sites/2481/Place-du-Martroi |
| Inrap Loire | Inrap (S. Caillé, E. Miéjac, C. Sauvé, V. Serna), notice « La Loire », atlas d'Orléans, 2014. https://multimedia.inrap.fr/userdata/atlas_pdf/9/print_2486.pdf |
| Jourd'heuil | J.-V. Jourd'heuil, fiche « Saint-Aignan d'Orléans », *Base des collégiales séculières de France (816-1563)*, Université de Limoges. https://collegiales.applirecherche.unilim.fr/?i=fiche&j=648 |
| Alix 2009 | C. Alix et J. Noblet, « Orléans. La salle des Thèses, 2, rue Pothier, bibliothèque de l'Université médiévale », *Bulletin monumental*, 167-4, 2009. https://www.persee.fr/doc/bulmo_0007-473x_2009_num_167_4_7332 |
| WP Sainte-Croix | Wikipédia, « Cathédrale Sainte-Croix d'Orléans » (d'après G. Chenesseau 1921, 2017 ; Lefèvre-Pontalis et Jarry 1905, *Mém. Soc. archéol. et hist. de l'Orléanais* XXIX). https://fr.wikipedia.org/wiki/Cath%C3%A9drale_Sainte-Croix_d%27Orl%C3%A9ans |
| WP Pont | Wikipédia, « Pont des Tourelles » (d'après A. Collin, *Le pont des Tourelles à Orléans (1120-1760)*, 1895). https://fr.wikipedia.org/wiki/Pont_des_Tourelles |
| WP Siège | Wikipédia, « Siège d'Orléans (1428-1429) » (d'après le *Journal du siège*, éd. Charpentier et Cuissard, 1896). https://fr.wikipedia.org/wiki/Si%C3%A8ge_d%27Orl%C3%A9ans_(1428-1429) |
| OSM | OpenStreetMap : nœuds « Ancienne porte Bannier » (47,90230 N, 1,90407 E), « Pont médiéval » (47,89558 N, 1,90584 E), place du Général-de-Gaulle, rues des Carmes, de la Hallebarde, Notre-Dame-de-Recouvrance, des Hôtelleries. |

## Tableau des faits

| Fait (donnée VH7) | Verdict | Source | Changement |
|---|---|---|---|
| Date de la première accrue (bourg Dunois) : 1345 | **incertain** | SAMO (« début du XIVe s. ») ; Inrap De Gaulle (« au tournant du XIVe s. ») ; Petit 1987 (« milieu du XIVe s. ») ; Wikipédia Histoire d'Orléans (« vers 1356 », porte Renard finie avant 1390, Bannier vers 1392) ; Carron 2012 (murailles « probablement » après 1385, achevées en 1391 sauf le front de Loire, fini dans les années 1410) | 1345 gardé (milieu du siècle, entre les hypothèses) ; note de l'enceinte réécrite avec les quatre datations |
| Mur occidental de l'accrue vers 1,9008 E, porte Renart à l'ouest de la rue des Carmes (-708, 37) | **corrigé** | SAMO : l'enceinte « suit un vallon (rue Notre-Dame-de-Recouvrance) », de la porte Bannier une courbe concave jusqu'à la porte Renard (place De Gaulle, « place du Marché de la Porte-Renard »), puis une courbe convexe jusqu'à la Barre-Flambert ; Inrap enceintes et Inrap De Gaulle : porte Renart sous la place De Gaulle, au croisement des anciennes rues du Tabour et de la Hallebarde | Porte Renart déplacée à (-595, -3) (≈ 115 m à l'est), mur nord-ouest refait (Bannier → Renart concave, Renart → rue Notre-Dame-de-Recouvrance → Loire) ; le coin nord-ouest carré (-702, 95) disparaît ; quartiers `bourg_dunois`, `bourg_dunois_ouvert`, `faubourg_bannier`, `faubourg_renart` recalés. Enceinte : 39,6 ha et 2 657 m (sources : 37 ha, 2 590 m ; avant : 41,8 ha, 2 770 m) |
| Mur nord au sud du Martroi, porte Bannier (-461, 88) | **confirmé** | Inrap Martroi (porte sous la place du Martroi, courtine vers la tour du Heaume à l'est) ; Petit 1987 ; OSM « Ancienne porte Bannier » (-460, 97) | aucun |
| Fossé de l'accrue : 12 m | **corrigé** | Carron 2012 et Inrap De Gaulle (fossé de 14-15 m à la porte Renart) ; Petit 1987 (plus de 15 m, 6-7 m de profondeur) ; Inrap Martroi (jusqu'à 20 m) | `ditch_m` 15 |
| Tracé du castrum (rues Sainte-Catherine, Tour-Neuve, Bourdon-Blanc ; vestiges valentiniens) | **confirmé** | SAMO (forme quadrangulaire, front sud de 570 m le long de la Loire) ; Inrap enceintes (2 032 m, 25 ha) ; OSM « Remparts gallo-romains valentiniens » | aucun sur le tracé |
| Gabarit du castrum : haut. 8 m, tours tous les 40 m | **corrigé** | SAMO : courtine de 2,5 à 4 m d'épaisseur, « hauteur supposée d'une dizaine de mètres », tours de 8 m de diamètre « environ tous les 55 m », un peu plus hautes que la courtine, fossé de 10 m × 3,5 m | `height_m` 10, `tower_spacing_m` 55, `tower_height_m` 12 (castrum et anneau de 1345) |
| Nom de « porte Dunoise » (porte ouest du castrum) | **confirmé** | SAMO : « porte Dunoise à l'ouest, porte Parisie au nord et porte Bourgogne à l'est » | aucun (la graphie « Parisie » de SAMO et « Parisis » coexistent ; gardé « Parisis ») |
| Guichets Saint-Benoît et de Moi (front de Loire) | **incertain** | SAMO : « au moins six portes ou poternes » ; noms non vérifiés | aucun |
| Pont des Tourelles : 21 arches (contre 19 souvent cités) | **confirmé** | WP Pont (d'après Collin) : 21 arches à l'origine (14 + 7), 19 après les glaces de 1434-1435, 18 ensuite ; Inrap Loire : 18 à sa disparition (13 + 5) | aucun : 21 est juste pour 1340-1429 (le chiffre de 19 est postérieur à 1435) ; note précisée |
| Axe du pont (Châtelet (-364, -358) → Tourelles (-391, -700)) | **corrigé** | Inrap Loire : pont « dans le prolongement de la rue des Hôtelleries » ; OSM « Pont médiéval » (-408, -661) : l'ancien axe passait 20 à 30 m trop à l'est de ses propres repères | Pont (-394, -353) → (-409, -684), 331 m ; porte du Pont, Tourelles, boulevard et motte recalés sur l'axe ; angle 85,8° |
| Bastille Saint-Antoine en pierre, présente dès 1340 | **corrigé** | Inrap Loire et WP Pont : hospice et chapelle Saint-Antoine sur la motte dès l'origine du pont ; « dès 1417 », après Azincourt, les Orléanais y érigent « une grande bastille en bois » | Bastille : `from_year` 1417, gabarit `earthwork` fermé (levée palissadée) à l'aval du pont ; nouvelle entrée `chapelle_saint_antoine` (toutes années, hypothétique) |
| Fort des Tourelles (châtelet à quatre tours) | **incertain** | Aucune source trouvée sur la date de construction ni le nombre de tours ; SAMO et WP : séparé de la rive par un pont-levis, pris les 23-24 octobre 1428, repris le 7 mai 1429 | Description réécrite (tours, date non documentées) ; gabarit gardé |
| Boulevard des Tourelles, terre et bois en 1428 | **confirmé** (nature) / **corrigé** (date) | WP Pont : « ouvrage de terre et de bois », pris le 22 octobre 1428 ; boulevards établis « dès 1417 » ; SAMO : ravelin maçonné en 1591-1592 | `from_year` 1404 → 1417 |
| Boulevards des portes à partir de 1404 (« Inrap ») | **corrigé** | SAMO : « premiers boulevards en avant des portes, entre 1417 et 1420 » (Bannier, Parisie, Renard, Bourgogne) ; Carron 2012 : « à partir de 1417 », porte Renart entre 1418 et 1422 (fossé en fer à cheval d'environ 55 m, 13-15 m de large, 6-7 m de profondeur) | Bourgogne, Parisis, Bannier : 1417 ; Renart : 1418, déplacé devant la nouvelle porte (-620, 3), sur l'axe de la rue des Carmes |
| Saint-Aignan rasée en 1358-1359, reconstruite vers 1420, rasée en 1428 | **confirmé** (1359, 1428) / **incertain** (1420) | Jourd'heuil : abattue pendant l'hiver qui suit 1358 contre Robert Knolles, reconstruction interdite jusqu'en 1376, abattue « à titre préventif » en 1428 ; WP Siège : « huit ans après sa reconstruction en 1420 » | Dates gardées (`until_year` 1358 ; 1420-1428) ; descriptions complétées (1376) |
| Saint-Euverte, debout de 1340 à 1428 | **incertain** | WP Saint-Euverte (sans source) : démolie en 1358 puis en 1428 ; date de la reconstruction intermédiaire inconnue | Description complétée ; dates gardées faute de date de reconstruction |
| Chevet de Sainte-Croix : 58 × 46 m, voûte 30 m | **incertain** (emprise) / **corrigé** (voûte) | WP Sainte-Croix : vaisseau central de 13,94 m, 31,75 m sous clef (état actuel), cinq vaisseaux de 50,86 m hors œuvre ; longueur du chevet médiéval non publiée | `vault_m` 32 ; longueur et largeur gardées |
| Sainte-Croix : « chevet seul » en 1340 et 1429, ni transept, ni nef, ni façade ; terrain de la nef en chantier | **corrigé** (correction majeure) | WP Sainte-Croix (d'après Chenesseau, Lefèvre-Pontalis) : chantier gothique commencé par le chevet (1287), nouveau chœur au XIVe s., travaux ralentis pendant la guerre de Cent Ans, croisée reprise dans la seconde moitié du XVe s., deux travées de nef au XVIe s. ; en 1568 subsistaient encore les façades des bras romans du transept et les tours romanes ; la façade romane à deux tours n'est démolie qu'en 1739. La cathédrale romane (nef de sept travées à bas-côtés simples, transept, façade harmonique) était donc encore debout à l'ouest du chevet | Nouvelle entrée `sainte_croix_romane` (`church`, 80 × 24 m, tour occidentale, probable ; transept non figuré) ; espace libre `chantier_nef` supprimé ; description du chevet réécrite |
| Châtelet : 48 × 36 m | **incertain** | SAMO : attesté dès le XIIe s., contrôle la tête nord du pont et un angle de l'enceinte ; aucune dimension publiée trouvée | aucun (gabarit restitué, `probable`) |
| Tour Neuve (grosse tour ronde, angle sud-est) | **confirmé** | SAMO : « au tout début du XIIIe s. », grosse tour circulaire à l'extrémité est de la façade fluviale, fossé maçonné | aucun |
| Faubourgs et églises hors les murs rasés du 8 novembre au 29 décembre 1428 | **confirmé** | WP Siège (d'après le Journal du siège) ; Jourd'heuil (Saint-Aignan) | aucun |
| Couvent des Augustins debout jusqu'en 1429 | **corrigé** | WP Siège et notices du Journal du siège : couvent abattu en octobre 1428 ; les Anglais fortifient ses ruines (bastille prise le 6 mai 1429) | `augustins` : `until_year` 1428 ; nouvelle entrée `bastille_augustins` (`earthwork`, 1429 seulement, hypothétique) |
| Bastille Saint-Loup (prise le 4 mai 1429) | **confirmé** (date) / **incertain** (nature de l'église) | WP Siège ; ville de Saint-Jean-de-Braye et montjoye.net : abbaye fondée en 1249 (source faible) | aucun (« prieuré » gardé) |
| Bastille Saint-Jean-le-Blanc (abandonnée le 6 mai 1429) | **confirmé** | WP Siège | aucun |
| Saint-Laurent-des-Orgerils, Saint-Paterne (« Saint-Pouair »), Saint-Marceau rasées en 1428 | **confirmé** | WP Siège (bastilles Saint-Laurent et Saint-Pouair ; démolition de tous les faubourgs) | aucun |
| Salle des Thèses à partir de 1411 | **corrigé** | Alix 2009 : trois textes de 1411 sur les travaux ; en 1414 les fondations et une partie des élévations seulement sont achevées | `from_year` 1415 |
| Saint-Paul (position (-629, -131)) | **incertain** | OSM « Restes de l'église Saint-Paul » vers (-652, -103), soit ≈ 35 m | aucun (écart sous la tolérance d'une restitution) |
| Emprises `probable`/`hypothetical` restantes (Saint-Donatien, Saint-Pierre-Empont, Saint-Étienne, Saint-Vincent, cloître Saint-Aignan, halles du Châtelet, grève) | **incertain** | Aucune source d'emprise trouvée ; existence des paroisses au XIVe s. non contestée | aucun |

## Corrections majeures

1. **Mur occidental de l'accrue** : la porte Renart est sous la place De Gaulle (Inrap, SAMO), non
   150 m plus à l'ouest ; le mur suit une courbe concave depuis la porte Bannier, puis le vallon de
   la rue Notre-Dame-de-Recouvrance. L'enceinte de 1345 tombe de 41,8 à 39,6 ha (sources : 37 ha).
2. **Cathédrale** : en 1340 comme en 1429, le chevet gothique est raccordé à la cathédrale
   romane (nef, transept, façade à deux tours), qui n'est remplacée qu'aux XVe-XVIe s. (et la
   façade en 1739) : il n'y avait pas de « terrain de la nef à bâtir ».
3. **Boulevards** : à partir de 1417 (porte Renart 1418-1422), non 1404.
4. **Motte Saint-Antoine** : chapelle et hospice dès le XIIe s., bastille en bois seulement dès 1417.
5. **Pont** : 21 arches confirmées pour 1340-1429 ; axe déplacé de 20-30 m vers l'ouest sur la rue
   des Hôtelleries et les vestiges du pont médiéval.

## Points ouverts

- Date de l'accrue : les sources vont du début du XIVe s. à 1391 ; si l'orchestration préfère
  suivre l'étude archéologique la plus récente (Carron 2012), passer `from_year` à 1391 (le bourg
  Dunois resterait alors défendu par fossé et palissade de 1340 à 1390).
- Collin (1895) et le *Journal du siège* (1896) n'ont pu être consultés directement (Gallica
  protégé par une vérification anti-robot) : chiffres repris par les notices qui les citent.
- Gabarits du Châtelet, des Tourelles et du chevet (longueur, largeur) : aucune dimension publiée
  trouvée ; les plans anciens de Gallica (contrôle humain) restent le meilleur recours.
- Tours de l'accrue nommées par SAMO (Heaume, Michau-Quanteau, Barre-Flambert) non figurées
  individuellement ; transept roman non figuré.
