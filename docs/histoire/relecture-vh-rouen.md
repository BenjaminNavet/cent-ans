# Relecture historique — Rouen vers 1340 (ville 1:1, VH4) — 26 septembre 2026

Relecture de `data/landmarks_v2/rouen.json` (ADR 0078, `docs/landmarks-v2.md`) : faits marqués
`probable` ou `hypothetical` d'abord, puis dates clés et gabarits. Les sources non commerciales ou
sous droits (Wikipédia, sites d'érudits, notices) servent aux faits seulement ; rien n'en est extrait
(`extracted: false` dans `sources`). Positions vérifiées en projetant en EPSG:3035 (pyproj) les objets
OSM correspondants (`historic=*`).

## Bilan

| Verdict | Nombre |
|---|---|
| Confirmé | 14 |
| Corrigé (y compris les lignes 21 et 23, confirmées sur la date mais corrigées sur un point) | 10 |
| Incertain (laissé `probable` / `hypothetical`, expliqué) | 7 |

**Correction majeure** : l'enclos oriental (Saint-Nicaise, Saint-Vivien, Martainville, portes
Saint-Hilaire et Martainville) n'existe pas en 1340. L'accrue est décidée en **1346** par Philippe VI
et achevée au début du XVe s. L'enceinte est désormais découpée en deux anneaux datés, comme
Orléans : `enceinte_xiiie` (jusqu'en 1345, front est hypothétique) et `enceinte` (à partir de 1346,
tracé des boulevards).

## Tableau des faits

Abréviations des sources :
- **Tanguy** : J. Tanguy, *Rouen ville forte*, rouen-histoire.fr, 2006-2007 (<https://rouen-histoire.fr/Fortifs/index.htm>, pages `XIIe.htm`, `XIVe.htm`, `Portes.htm`, `../Ponts/Mathilde.htm`).
- **Follain** : É. Follain, « La dernière enceinte de Rouen », *Patrimoine normand* (<https://patrimoinenormand.com/article-148959-fortifications-rouen.html>).
- **POP-GH** : Inventaire général Normandie, « Beffroi dit Gros Horloge », IA00021886 (<https://pop.culture.gouv.fr/notice/merimee/IA00021886>).
- **POP-PM** : Inventaire général Normandie, « Pont de l'Impératrice Mathilde… », IA00021781 (<https://pop.culture.gouv.fr/notice/merimee/IA00021781>).
- **WP** : articles de Wikipédia FR cités (faits recoupés quand c'était possible).

| # | Fait restitué (avant) | Verdict | Source | Changement fait |
|---|---|---|---|---|
| 1 | Enclos oriental (Martainville) fermé en 1340, « probable, à vérifier » | **Corrigé** | Tanguy (`XIVe.htm` : « à l'injonction de Philippe de Valois… on en profita pour enclore les quartiers de l'est, de part et d'autre du Robec ») ; Follain (« entrepris à partir de 1346… ne prendront fin qu'au début du XVe siècle ») | `enceinte` datée `from_year: 1346`, renommée « après l'accrue orientale (1346-début XVe s.) » ; nouvel anneau `enceinte_xiiie` `until_year: 1345` |
| 2 | Porte Saint-Hilaire en 1340 | **Corrigé** | Fouille Inrap 2021 (place Saint-Hilaire, surveillance de travaux) : porte de l'enceinte du XIVe s. (compte rendu : patrimoinenormand.com, article 148289) ; Tanguy (`Portes.htm`) | Porte seulement sur l'anneau de 1346 ; absente de `enceinte_xiiie` |
| 3 | Porte Martainville en 1340 | **Corrigé** | Tanguy (`Portes.htm` : route de Paris, « reconstruite au début du XVe siècle ») ; même accrue | Idem |
| 4 | Front est de l'enceinte avant 1346 | **Incertain** | Tanguy (`XIIe.htm` : murs nord et est englobant Saint-Ouen et le quartier Saint-Maclou ; base du mur dans l'ancien jardin de Saint-Ouen) ; WP « Jardin de l'Hôtel-de-Ville » (vestiges de la clôture du monastère, commune à celle de la ville au XIIIe s., mis au jour dans les années 1970 lors de l'extension vers la rue de l'Épée) | Tracé hypothétique : jardin de Saint-Ouen (vers la rue de l'Abbé-de-l'Épée) → rupture rue des Faulx / rue Saint-Vivien → rue du Ruissel → Seine à l'est de la rue du Rempart-Martainville. Deux portes restituées : « Porte Saint-Vivien » (nom cité par une seule source secondaire, remparts-de-normandie.eklablog.com) et « Porte de Martainville (première) » (nom non attesté) |
| 5 | Saint-Vivien : dans ou hors les murs en 1340 | **Incertain** | WP « Église Saint-Vivien de Rouen » (paroisse *intra-muros* dès 1230, d'après Lemoine et Tanguy, *Rouen aux 100 clochers*) contre Tanguy (`XIIe.htm`, `XIVe.htm` : quartiers du Robec enclos seulement sous Philippe VI) | Laissé hors les murs, contradiction écrite dans la `note` de `enceinte_xiiie` et la description de l'église |
| 6 | Front nord (Cauchoise-Bouvreuil-Beauvoisine) déjà sur la ligne des boulevards avant 1346 | **Incertain** (probable) | Rue des Fossés-Louis-VIII : fossés de 1200, cédés par Louis VIII en 1224 (donc mur reporté plus au nord dès le XIIIe s.) ; Tanguy (ville atteignant à l'ouest et au nord-ouest la limite des boulevards sous les Plantagenêts). Une source secondaire range Beauvoisine parmi les portes de 1346 (non recoupé) | Gardé sur les deux anneaux |
| 7 | Pont Mathilde « vers 1160 » | **Confirmé** | POP-PM (« construit entre 1151 et 1167 », piles démolies 1659-1661) ; Tanguy (vers 1160, Barbacane) | `note` précisée |
| 8 | Pont : 13 arches | **Incertain** | 13 arches « dont 5 surélevées » selon plusieurs auteurs ; 12 selon Tanguy (`Ponts/Mathilde.htm`) | Laissé à 13, écart noté |
| 9 | Maisons et moulin sur le pont en 1340 | **Incertain** | Le Lieur (1525) seulement | Inchangé (déjà signalé comme restitution) |
| 10 | Cathédrale : 137 m | **Corrigé** | WP « Cathédrale Notre-Dame de Rouen » (144 m hors œuvre) ; structurae / oraedes (136,86 m dans œuvre) | `length_m` 137 → 144 (gabarit = emprise extérieure) ; test pytest mis à jour |
| 11 | Tour Saint-Romain : étage supérieur 1468-1477, plus basse en 1340 | **Corrigé** (date) / hauteur **incertaine** | WP (étage supérieur 1468-1478, 82 m aujourd'hui) | Description : 1468-1478 ; hauteur 58 m + toit laissée, dite restituée |
| 12 | Tour-lanterne 51 m, flèche de bois brûlée en 1514 | **Incertain** (hauteur) ; incendie **confirmé** | WP (flèche incendiée le 5 octobre 1514 ; 51 m = hauteur **sous voûte** de la lanterne) | Description précisée ; `crossing.height` 51 laissé (la tour hors œuvre était plus haute, valeur inconnue) |
| 13 | Tour de Beurre absente | **Confirmé** | WP (1485-1506) | — |
| 14 | Saint-Ouen : chœur 1318-1339 | **Confirmé** | WP « Abbatiale Saint-Ouen » (début 1318, abbé Jean Roussel dit Marc d'Argent ; chœur achevé vers 1339 ; nef achevée en 1537) | Description précisée |
| 15 | Saint-Ouen : nef romane debout en 1340 | **Confirmé** (gabarit probable) | WP (chantier 1062, consécration 1126, incendies 1136 et 1248 ; fouilles de 1885 : dimensions comparables à l'édifice gothique) | Description précisée, `probable` gardé |
| 16 | Château de Philippe Auguste | **Confirmé** (1204-1210) | WP « Château de Rouen » ; Réunion des musées métropolitains | Description |
| 17 | Donjon (tour Jeanne-d'Arc) : rayon 7,5 m, 30 m, position | **Corrigé** | WP (≈ 14 m de diamètre, ≈ 35 m) ; OSM (tour Jeanne-d'Arc à (44, 698)) | `keep` : rayon 7, hauteur 35, recentré sur la tour OSM (écart 13 m → 0) |
| 18 | Emprise du château (anneau ≈ 190 × 170 m) | **Incertain** | WP (enceinte ≈ 90 × 90 m, dix tours dont trois demi-tours, fossé 15 m, grande basse-cour sur l'amphithéâtre) ; tour de la Pucelle (OSM) sur le front nord de l'anneau | Anneau gardé (il englobe la basse-cour), réserve écrite dans la description |
| 19 | Halles de la Vieille-Tour (`probable`) | **Confirmé** | WP « Château de Rouen », « Place de la Haute-Vieille-Tour » (palais ducal rasé en 1204 ; halles vers 1259 sous Louis IX : toiles, grains, draps ; marché attesté 1262) ; OSM (place de la Haute-Vieille-Tour) | Description ; `probable` gardé (un seul volume pour trois halles) |
| 20 | Clos des Galées « 1294 », emprise « probable » | **Corrigé** | WP « Clos aux galées » ; A. Chazelas, *Documents relatifs au Clos des galées de Rouen… 1293 à 1418* (CTHS) ; Ch. de Beaurepaire, BEC 1865 (Persée) ; Tanguy (clos plus à l'est, loin de la rive, relié par un canal) | Nom sans date, description « premiers documents 1293, fondation souvent datée de 1294 » ; `until_year: 1418` (détruit à l'approche des Anglais) ; `clos_des_galees_1451` (rebâti) ; `certainty` abaissée à `hypothetical` |
| 21 | Abbaye Sainte-Catherine, 1030 | **Confirmé** ; fin **corrigée** | WP « Gosselin d'Arques », « Abbaye Sainte-Catherine-du-Mont » (fondée en 1030, fortifiée, détruite en 1597) ; OSM (site à 1 m de la restitution) | `until_year: 1597`, description |
| 22 | Beffroi communal jusqu'en 1382, emplacement `hypothetical` | **Confirmé**, certitude **relevée** | POP-GH (« beffroi primitif démoli en 1382 à la suite de l'émeute de la Harelle ; reconstruit sur les vestiges de l'édifice primitif ») | `certainty` → `probable` (site attesté, gabarit restitué) |
| 23 | Gros-Horloge à partir de 1389, toit en lanterne | **Confirmé** (1389) ; toit **corrigé** | POP-GH (1389-1398, Jean de Bayeux ; horloge de Jourdain Delestre et Jean de Felains ; dôme de 1711 « à la place de la flèche primitive ») | `top` lanterne → pyramide ; description. `from_year` 1389 gardé (début du chantier, achevé en 1398) |
| 24 | Aître Saint-Maclou à partir de 1348 | **Confirmé** (avec réserve) | WP « Aître Saint-Maclou » (cimetière de la peste de 1348, première mention en 1362 ; galeries 1526-1533) | — (l'aître est dans l'enceinte d'avant 1346) |
| 25 | Vieux-Palais absent | **Confirmé** | Henri V, à partir de 1419-1420 (déjà dans le test) | — |
| 26 | Saint-Maclou : église antérieure | **Confirmé** | Église actuelle 1437-1517 (WP) | Description |
| 27 | Saint-Vivien (église) | **Confirmé** | WP (reconstruite à partir de 1338, consacrée le 24 mai 1358) | Description |
| 28 | Positions des monuments tirées d'OSM | **Confirmé** | OSM projeté : tour du Beffroi (−252, 175) contre (−250, 175) ; Saint-Laurent (83, 464) ; Saint-Vincent (−430, 60) ; Saint-Vigor (−439, 549) ; Saint-Nicolas (75, 151) ; Sainte-Catherine (1160, −954) | — |
| 29 | Nef de la cathédrale : voûtes 28 m, vaisseau central 11 m | **Confirmé** | WP ; oraedes (nef 60 m × 24,2 m, vaisseau central 11,3 m, 28 m) | — |
| 30 | Période affichée | **Corrigé** | — | `period` : « avant l'accrue orientale de 1346, la peste de 1348… » |
| 31 | Fossés Louis VIII (nord de la ville) hors du tracé de 1340 | **Confirmé** | Rue des Fossés-Louis-VIII : fossés cédés en 1224 (rouen-histoire.com, fiche 252 ; P. Halbout, *Archéologie médiévale* 12, 1982, notice de fouille) | — |

## Ce qui reste ouvert

- **Front est d'avant 1346** : tracé et portes hypothétiques. Sources à consulter : D. Pitte et al.,
  « Les fouilles du rectorat (1992) : contribution à la connaissance des enceintes de Rouen », in
  *Les enceintes urbaines (XIIIe-XVIe s.)*, CTHS (téléchargement soumis à inscription, non fait) ;
  J. Munier, *Les défenses militaires de la dernière enceinte de Rouen* (1992) ; *Carte archéologique
  de la Gaule 76/2 — Rouen* (2005) ; J. Decoux et G. Gaillard, *Le quartier Martainville de Rouen*
  (Inventaire, 2011).
- **Quartier `ville_close`** : son polygone couvre encore l'est enclos en 1346 ; en 1340 ces maisons
  (quartier drapier déjà dense) sont hors les murs mais gardent le mélange de maisons de la ville.
  Les quartiers ne sont pas datés dans le format v2.
- **Jardins de Saint-Ouen** : le polygone déborde à l'est du mur d'avant 1346 (terres de l'abbaye
  hors les murs, plausible mais non vérifié).
- **Clos des Galées** : position à reprendre avec Chazelas ou Beaurepaire (plus à l'est, canal
  d'accès).
- **Château** : emprise réelle de l'enceinte (≈ 90 × 90 m) et de la basse-cour à reprendre avec un
  plan d'érudit (Quenedey, Duranville).
- Hauteurs anciennes de la tour Saint-Romain, de la tour-lanterne et de la flèche de 1340 : inconnues.

## Vérifications

- `uv run --project tools pytest tools/tests/test_landmarks_v2.py` : 32 réussis (test
  `test_rouen_1340_facts` mis à jour : longueur 144 m, deux anneaux datés 1345/1346, portes
  Saint-Hilaire et Martainville absentes de l'anneau ancien).
- `godot --headless --path game --script res://tests/vh4_landmarks_test.gd` : OK (1340 : un anneau,
  9 portes, 59 tours, 26 monuments).

## État (reprise)

Terminé sur la branche du worktree `worktree-agent-a72aaedd56655f591` ; à fusionner par
l'orchestrateur. Script d'édition non versionné (modifications faites via `write_city`, format du
fichier inchangé).
