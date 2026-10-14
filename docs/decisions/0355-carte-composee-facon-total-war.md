# 0355 — Carte composée façon Total War

Statut : accepté (14/10, choix du joueur) ; forme validée par le joueur sur la maquette 2D du
lot C0 le 14/10 : périmètre de fusion `realm` (208 provinces, 400 lieux), cartogramme strength 0,5,
lissage 200 km, densité de la mer 1,0. Remplacera, à la fusion, la carte à géographie
réelle (ADR 0019, 0036, 0115-0116 pour l'emprise, 0138 et 0158 pour l'échelle 1:1 et le relief).
Spécification : `docs/design/2026-10-14-carte-composee.md`.

## Contexte
Après le relief exagéré (ADR 0353), le joueur veut aller jusqu'à une réinterprétation de la
géographie comme les cartes Total War : sélection de villes, repositionnement, simplification.
Choix : emprise Europe + Méditerranée (Oural et steppe lointaine retirés), ≈ 400 lieux jouables
(≈ 150-200 provinces), géographie reconnaissable mais caricaturée. Les batailles gardent le relief
réel de leur site.

## Décision
- La carte est **composée** : description vectorielle d'auteur (déformation, côtes, fleuves, lacs,
  crêtes, lieux) compilée en le même contrat que l'étape 0 actuelle de `geo build`
  (`heightmap.png`, `land_mask.png`, `rivers.geojson`, `coastline.geojson`, `map.json`) ; l'aval de
  la chaîne est conservé.
- **Déformation** réel → carte par cartogramme de diffusion sur la densité des lieux retenus
  (lieux régulièrement espacés à l'écran), corrigée par des épingles d'auteur ; nouveau CRS
  `composed` ; une seule fonction de projection par langage.
- **Relief procédural** sur des lignes de crête d'auteur (plaines plates, massifs, cols creusés) ;
  la pyramide de relief fin réel est abandonnée pour la campagne, détail rapproché procédural.
- **Rasters thématiques** (couverture, climat, forêts, zones humides, agraire) rééchantillonnés par
  la déformation inverse puis généralisés.
- Chaque lieu garde sa **vraie latitude/longitude** pour les règles qui en dépendent (météo, peste,
  sécheresse) et pour les batailles (relief et horizon du site réel).
- **Identifiants conservés** quand une province ou un lieu survit ; table ancien → nouveau et outil
  de réécriture automatique des références.
- Sauvegardes incompatibles ; développement sur la branche longue `cc-carte-composee`.

## Conséquences
- Perte de la fidélité kilométrique, du relief fin réel rapproché (pyramide, LiDAR, fleuves fins),
  d'≈ 1 250 lieux mineurs et d'≈ 20 factions de l'Est.
- Bandes seed bots, `points_per_step`, km des fleuves, objectifs de victoire et tests à
  coordonnées à recalibrer (lot C6).
- Les chantiers carte parallèles (GR, CV…) doivent se coordonner avec la branche.
