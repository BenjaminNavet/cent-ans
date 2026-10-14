# 0349 — Ingestion fidèle des modèles Meshy

Statut : accepté (13/10).

## Contexte
Le pipeline DN (`dn-ingest`, ADR 0211-0212) a été écrit pour les modèles TRELLIS : albédos sombres
étalonnés vers la palette (luminance 0,18-0,35, saturation plafonnée), socle parasite supprimé, îlots de
débris retirés, LOD0 réduit à 15 000 triangles ; le jeu éclaircit ensuite l'albédo ×2,4
(`albedo_gain` de `dn_campaign_models.json`). Appliqué aux villes et châteaux Meshy (MX), ce traitement
les dégradait : couleurs assombries puis blanchies, pelouse et ponts du socle arrachés, éclats
flottants, coutures déchirées par la réduction (planche du 13/10 : généré / ingéré / matériau du jeu).
Le joueur veut voir en campagne exactement le modèle généré.

## Décision
- `cent-ans dn-ingest --faithful` : pas d'étalonnage, pas de réduction de texture, socle et îlots
  conservés, LOD0 = maillage brut (≈ 30 000 triangles pour une ville meshy-6-lite) ; LOD1-2 restent
  réduits. Le manifeste note `faithful: true`.
- Les 31 villes et châteaux Meshy de la campagne sont ingérés ainsi, avec `albedo_gain: 1.0` par entrée
  dans `dn_campaign_models.json` (les modèles TRELLIS gardent le gain 2,4 des défauts).
- Un modèle Meshy raté se corrige à la génération (image source plus simple), pas à l'ingestion.

## Conséquences
- Rendu en campagne identique au modèle généré, à la rugosité près (constante 1).
- LOD0 deux fois plus lourd pour ces lieux (pas d'écart de temps d'image mesurable à 15 000, à
  revérifier au banc si la carte ralentit).
- Le socle généré est posé tel quel sur le relief.
