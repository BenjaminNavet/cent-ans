# VH8 — Bruges vers 1340 à l'échelle 1:1 (format v2, ADR 0078)

Branche `feat/rs-g-cities` (worktree partagé avec trois autres villes). Référence :
`docs/landmarks-v2.md`, exemple Rouen. Fichiers du lot : `data/landmarks_v2/bruges.json`,
`tools/tests/test_landmarks_v2_bruges.py`, ce suivi. Script d'auteur non versionné (scratchpad).

## État : terminé (à relire et fusionner par l'orchestrateur)
- Origine sur le beffroi, EPSG:3035 [3848306, 3143840] (76 m de l'ancre v1), `extent_m` 1600.
- Enceinte `vesten` (levée de terre de 1297, 7,1 km restitués) : axe du fossé intérieur d'après
  OSM, levée 20 m à l'intérieur, 6 m. 19 états de portes datés : en 1340 passages bas (portes
  démantelées vers 1328) ; Gentpoort 1361 / détruite 1382 / 1401 ; Kruispoort 1366 / 1382 / 1401 ;
  Boeveriepoort 1366 ; Smedenpoort 1367 ; Ezelpoort 1369 ; Katelijnepoort 1401 ; Dampoort (trois
  portes, non datées, positions hypothétiques). Murs de pierre de 1398 (Begijnenvest, cinq tours)
  et 1399 (Minnewater-Katelijnepoort).
- 28 monuments datés avec `certainty` : beffroi en trois états (≤ 1344, 1345-1485, ≥ 1486),
  halles, Waterhalle (≤ 1787), Ghyselhuus (≤ 1375), hôtel de ville (≥ 1376), Saint-Basile,
  Saint-Donatien, Steen, Notre-Dame et sa tour, Saint-Sauveur (chœur, nef romane ≤ 1358, nef
  rebâtie ≥ 1359), Saint-Jacques, Saint-Gilles, Saint-Jean, béguinage et son église, Potterie,
  Dominicains, Franciscains, Augustins, Eekhout, Poertoren (≥ 1398), tour est du Minnewater
  (1400-1621), waterhuis de 1288.
- Eaux : `fine_rivers: []` ; fossé des vesten (24 m, restitué) et 11 reien tracés d'après OSM
  (largeur médiane tirée des surfaces d'eau OSM), Kraanrei prolongée à la main sous la Waterhalle.
  Exclus : Coupure (1751), canaux Gand-Bruges et Bruges-Ostende (XVIIe s.), Boudewijnkanaal,
  Afleidingsvaart, Damse Vaart, Handelsdok.
- Quartiers : ville de la première enceinte (anneau des reien, 67 ha, densité 0,92) et accrues de
  1297 (densité 0,55) ; pas de faubourg. Espaces : Markt, Burg, pré du béguinage.
- Rues : 778 tronçons OSM (vesten, « -laan » du XIXe s., Coupure, rues du quartier nord-ouest
  loti au XIXe-XXe s. exclues).
- Tests : `test_landmarks_v2.py` + `test_landmarks_v2_bruges.py` : 59 réussis.

## Manques (restitutions hypothétiques)
- Hauteurs du beffroi en 1340 et 1346, de la tour de Notre-Dame (sans flèche), des portes,
  de la levée avant 1702 ; largeur du fossé en 1340 ; second fossé extérieur (non tracé).
- Gabarits de Saint-Donatien, du Steen (place : côté ouest du Burg), du Ghyselhuus, de
  l'hôtel de ville, de Saint-Basile, des nefs de 1340 (Notre-Dame, Saint-Sauveur, Saint-Jacques,
  Saint-Gilles) ; date d'achèvement de la nef de Saint-Sauveur après 1358.
- Emprises des couvents (Dominicains, Franciscains, Augustins, Eekhout) ; Carmes (Carmersstraat,
  place inconnue) et ancienne Sainte-Walburge (place inconnue) omis.
- Première enceinte : portes intérieures (Vlamingpoort, Ezelpoort/Sint-Jacobspoort,
  Koetelwijkpoort, Oude Molenpoort, Noord- et Zuidzandpoort, Mariapoort) : présence en 1340 et
  dates de démolition non trouvées, omises.
- Dampoort : positions et dates des trois portes. Tracé nord-ouest de l'enceinte (fossé comblé
  sous la Koningin Elisabethlaan et le Komvest) : probable, non sourcé.
- Ponts : aucun (petits ponts des reien ; Vlamingbrug de pierre dès 1331 non placée).

## Format (points manquants, contournés)
- Pas de levée de terre linéaire (enceinte = mur maçonné bas) ni de porte « passage » sans
  châtelet (passage = porte basse de 7 m).
- `belfry` : pas de flèche haute ni d'octogone (tour de Notre-Dame sans flèche de 1440,
  octogone du beffroi en lanterne).
- `waters` non datés (Kraanrei voûtée en 1787 : hors période, sans effet).

## Prochaine étape
Relecture historique (`docs/histoire/relecture-vh-bruges.md` à écrire), section Bruges dans
`docs/landmarks-v2.md`, contrôle en jeu (`vh4_shots.gd --city=bruges`) par la session principale.
