# Relecture historique — Calais vers 1340 (ville 1:1, VH8) — 28 septembre 2026

Relecture indépendante de `data/landmarks_v2/calais.json` (ADR 0078, `docs/landmarks-v2.md` ;
suivi de l'auteur : `docs/wip/rs-g-calais.md`). Méthode : faits marqués `probable` ou
`hypothetical` d'abord, puis dates clés (1337-1453) et gabarits. Source de référence : la
synthèse du service régional de l'archéologie (SRA 2025, texte lu en entier) ; POP, Greaves
(1918) et Wikipédia FR/EN recoupées en appoint. Faits seulement (`extracted: false`). Positions
vérifiées en projetant en EPSG:3035 (`lonlat_to_local`) les objets OSM correspondants (Overpass,
28 septembre 2026).

## Bilan

| Verdict | Nombre |
|---|---|
| Confirmé | 13 |
| Corrigé | 2 |
| Incertain (laissé `probable` / `hypothetical`, expliqué ; lignes 8 et 22 confirmées en partie) | 13 |

Pas de correction majeure : le fichier suit fidèlement ses sources et signale lui-même ses
restitutions. Corrections : date du mariage de Richard II à Saint-Nicolas (4 novembre 1396, non le
1er) et longueur du transept de Notre-Dame (46 m, non 52) ; descriptions de la tour du Guet et du
Rysbank complétées (POP, SRA), note de l'enceinte complétée. Le point faible reste le **tracé de l'enceinte** (fronts est et ouest) et la
**surface de la ville** : les sources discordent (≈ 44 ha, 52 ha, ≈ 81 ha) et aucun plan
d'érudit n'a pu être consulté.

## Sources principales

| Abr. | Référence |
|---|---|
| SRA 2025 | I. Speurt, J. Fossaert, J.-J. Tronquoy, L. Quintard-Tardie, *Calais, archéologie d'un territoire fortifié*, Archéologie des Hauts-de-France 41, DRAC-SRA, 2025. <https://www.culture.gouv.fr/mc/content/download/384538/pdf_file/41_SRA_Calais_version-en-ligne_def.pdf> |
| Vauban | Réseau des sites majeurs de Vauban, *Inventaire… Calais*, 2015. |
| Greaves | D. Greaves, « Calais under Edward III », dans G. Unwin (éd.), *Finance and Trade under Edward III*, 1918 (British History Online). |
| POP | Mérimée PA00108246 (Notre-Dame : XIVe s., classée en 1913) ; PA00108248 (tour du Guet : XIIIe s., « ancien phare », inscrite en 1926, classée en 1931). |
| WP FR | Wikipédia FR : Église Notre-Dame de Calais, Citadelle de Calais, Histoire de Calais. |
| WP EN | Wikipédia EN : Isabella of Valois, Calais, Siege of Calais (1346-1347), Fort Risban. |
| OSM | OpenStreetMap : tour du Guet (voie 57175436, `height` 39), Notre-Dame (104688596), fort Risban (187719034), tour Carrée (541449330), citadelle, boulevard du 8-Mai, esplanade Jacques-Vendroux. |

## Tableau des faits

| # | Fait restitué (avant) | Verdict | Source | Changement fait |
|---|---|---|---|---|
| 1 | Enceinte de Philippe Hurepel en 1228 | **Confirmé** | SRA 2025 (« En 1228, le comte de Boulogne Philippe Hurepel… décide de ceindre la ville d'un rempart », mur flanqué de petites tours et fossés extérieurs) ; WP FR Histoire (1224, sans source) | — (écart 1224 déjà noté) |
| 2 | Mur de 2,50 m, tour carrée de 6,50 m | **Confirmé** | SRA 2025 (boulevard du 8-Mai, 2017 : section de 55 m, 2,50 m, tour carrée de 6,50 m, large fossé) | — |
| 3 | Front nord restitué | **Confirmé** (approximativement) | OSM : le bras est du boulevard du 8-Mai, (−382, 20) → (−907, −117), passe à 10-15 m du front nord restitué (−330, 10) → (−700, −75) ; emplacement exact de la fouille de 2017 non publié | Note de l'enceinte |
| 4 | Front sud par la tour Carrée | **Confirmé** | OSM : tour Carrée à (−645, −500), sur la ligne restituée (≈ −495) | — |
| 5 | Fronts est et ouest, profondeur ≈ 500 m | **Incertain** | Vauban (1 100 × 400 m) ; Greaves (« about 200 acres », ≈ 81 ha, soit Calais-Nord) ; 52 ha ici ; tronçon de l'esplanade Jacques-Vendroux (2019) « pourrait » être médiéval, non situé | Écarts écrits dans la note |
| 6 | 44 tours, quatre portes à avant-portes sur terrée circulaire | **Confirmé** (source unique) | Vauban ; SRA 2025 : en 1550, état anglais, quarante tours et six portes | Note |
| 7 | Noms et positions des portes | **Incertain** | Noms anglais du XVIe s. seulement (Tudor Travel Guide) ; aucun nom français de 1340 | Inchangé (noms descriptifs, restitution déjà écrite) |
| 8 | Seconde enceinte et second fossé en place en 1340 | **Confirmé** (existence en 1346) / **incertain** (date, tracé) | SRA 2025 (« double enceinte et fossé doublé » en 1346) ; Greaves (deux murs, deux fossés inondables) ; WP EN Siege (« double moat ») | — |
| 9 | Château de 1228-1229 à l'angle nord-ouest, fondu dans l'enceinte | **Confirmé** | SRA 2025 (muraille carrée, donjon, porte vers la ville et porte vers Nieulay ; château au nord-ouest sur la restitution du siège) ; Greaves (« at the extreme north-west… merged in the town walls ») ; WP EN Siege (« its own moat ») ; WP FR Citadelle (1229, six tours et donjon, quatre tours reprises dans les bastions) | — |
| 10 | Emprise du château (≈ 76 m), donjon, « tour détachée » | **Incertain** | Aucune dimension publiée ; plan de 1693 : « Vieux Chasteau » dans la partie nord de la citadelle | Inchangé |
| 11 | Tour du Guet : XIIIe s., 39 m, position | **Confirmé** | POP PA00108248 (XIIIe s., « ancien phare ») ; OSM (`height` 39 ; centre à 1 m du point restitué) | Description et source POP ajoutées ; `probable` gardé (hauteur de 1340 inconnue) |
| 12 | Hôtel de ville de 1231, au sud de la place | **Incertain** | WP FR Histoire (« l'hôtel de ville qui datait de 1231 », sans source) ; Tudor Travel Guide (état Tudor) | Inchangé (`hypothetical`) |
| 13 | Notre-Dame du XIIIe s. sur le transept actuel, deux tours de façade au nord | **Incertain** | WP FR (église d'origine sur le transept, rectangulaire, deux tours de façade ; porte Saint-Pierre du bras nord flanquée de deux tours, partie la plus ancienne) ; POP PA00108246 (campagne principale au XIVe s.) ; article de P. Héliot (1947) non lu | Inchangé (`hypothetical`) |
| 14 | Reconstruction anglaise 1370, tour de croisée 1410 | **Incertain** | WP FR (seconde moitié du XIVe au XVIe s. ; tour-lanterne de 56 m au XVe s.) ; dates conventionnelles déjà signalées | Inchangé |
| 15 | Notre-Dame : 75 m (sans la chapelle de 1631), voûtes 18 m | **Confirmé** | WP FR (90 m hors œuvre, 88 m selon un autre passage ; chapelle elliptique de 18 × 9,75 m ; voûtes à 18 m) ; OSM (93 m est-ouest) | — |
| 16 | Transept de 52 m | **Corrigé** | WP FR (« 90 mètres de long sur 45,50 m de large ») ; l'emprise OSM (52 m nord-sud) inclut les tours saillantes du portail Saint-Pierre | `transept_m` 46 (phases anglaise et à tour) |
| 17 | Position de Notre-Dame | **Confirmé** | OSM (255, −262) contre (246, −260) et (250, −261) | — |
| 18 | Saint-Nicolas : mariage de Richard II le 1er novembre 1396 | **Corrigé** | WP EN Isabella of Valois (cérémonie le 31 octobre ; le 4 novembre 1396, « brought to the church of St. Nicholas in Calais where Richard married her ») | Description |
| 19 | Saint-Nicolas détruite pour la citadelle (première pierre 1564) | **Confirmé** | WP FR Citadelle (quartier et église rasés ; première pierre par le duc de Longueville en 1564 ; chapelle Saint-Nicolas de 1605) ; SRA 2025 (citadelle 1564-1574) | — ; position `hypothetical` |
| 20 | Étape des laines le 9 février 1363 | **Confirmé** | WP EN Calais (« On 9 February 1363 the town was made an English staple port ») ; 1363 recoupé (Merchants of the Staple) | — ; bâtiment et position restent `hypothetical` |
| 21 | Fortin du Rysbank `from_year` 1347 | **Incertain** | SRA 2025 (« première mention… 1346 dans les archives anglaises ») ; WP EN (bâti en novembre 1346) ; WP FR (installé fin avril 1347) | 1347 gardé ; description réécrite avec les trois datations |
| 22 | Tour de pierre du Rysbank (1400), position | **Confirmé** (nom, site) / **incertain** (date) | WP EN (Stone Tower renommée Lancaster Tower après 1400) ; SRA 2025 (tour de Lancastre et muraille polygonale à deux tours sous l'occupation anglaise ; fondations circulaires en 2019 ; rasée à mi-hauteur en 1586) ; OSM fort Risban à 23 m | Description (source SRA précisée) |
| 23 | Avant-portes sur terrée circulaire | **Incertain** | Vauban (seule source, sans date) : peut-être postérieures à 1340 | Inchangé (`hypothetical`) |
| 24 | Saint-Pierre, faubourg au sud | **Confirmé** | SRA 2025 (« Dès le XIIIe siècle, Saint-Pierre n'est plus qu'un faubourg de Calais ») ; Greaves (faubourgs à l'est, au sud et à l'ouest) | — |
| 25 | Havre au nord, langue de terre | **Incertain** | Greaves (« formed by a piece of land jutting east ») ; tracé du trait de côte de 1340 introuvable | Inchangé |
| 26 | Place du Marché au centre | **Confirmé** | Greaves (« In the centre was the market place ») | — |
| 27 | Églises omises : Saint-Jean / Maison-Dieu, Carmes (avant 1347), Augustins (1351) | **Incertain** | Greaves (existence) ; aucune position | Inchangé (omises) |
| 28 | Rues : trame OSM de la reconstruction | **Incertain** | Inventaire IA62005683 (remembrement, voies redressées et élargies) | Inchangé |

## Ce qui reste ouvert

- **Tracé et surface de l'enceinte** : fronts est et ouest restitués ; sources discordantes sur la
  surface. À reprendre avec L. Lenoir, *À la découverte des anciennes fortifications de Calais*
  (2001), les rapports de diagnostic du boulevard du 8-Mai (2017) et de l'esplanade
  Jacques-Vendroux (2019), et le plan de Calais de *The History of the King's Works* (vol. III).
- **Château** : emprise, donjon, tour détachée ; le plan de 1693 (contrôle humain) le place dans la
  partie nord de la citadelle.
- **Notre-Dame** : orientation et gabarit de l'église du XIIIe s. ; article de P. Héliot
  (*Bulletin monumental*, 1947, Persée) non lu ; dates de la reconstruction anglaise.
- **Rysbank** : 1346 ou 1347 pour le premier fort ; date de la tour de pierre.
- Noms français des portes en 1340 ; positions de Saint-Nicolas, de l'hôtel de ville, de l'Étape.

## Vérifications

- `uv run --project tools pytest -q tools/tests/test_landmarks_v2.py tools/tests/test_landmarks_v2_avignon.py tools/tests/test_landmarks_v2_calais.py`
  : 64 réussis. Aucun fait testé n'a changé à Calais (test inchangé).
- Fichier réécrit par `write_city` (format inchangé, aller-retour vérifié octet pour octet avant
  modification).
