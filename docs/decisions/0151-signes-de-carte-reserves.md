# 0151 — Un signe par ville ; signes d'évènement réservés à une couche

Date : 2026-10-02 · Lot TB2 (`docs/design/2026-10-02-campagne-tob.md` § 3)

## Contexte
La carte de campagne empilait plusieurs signes au même endroit : écu du détenteur au-dessus de
chaque nom, marteau « ⚒ » des chantiers, sceau de cire rouge des incidents (FK5, les « rosaces »),
pastille des sites de rencontre (CV3, les « étoiles »). Le plan TB2 demande un seul signe par ville
et par palier de zoom, et les rosaces et étoiles « seulement dans un mode de carte ». Or les sceaux
et les sites sont aussi des commandes de jeu (clic = décision, destination d'armée) et les incidents
n'ouvrent pas de fenêtre en début de tour : les masquer sans recours cacherait des décisions.

## Décision
- Le signe d'une ville est son écu. En vue moyenne et large, seuls les lieux majeurs le gardent
  (`shield.max_distance_by_rank` de `data/map/settlement_markers.json`) ; les autres n'ont que leur nom.
- Marteaux, sceaux et sites ne s'affichent plus sur la carte politique. Ils reviennent :
  - par la case « Signes » du menu des filtres (calque, comme les routes commerciales) ;
  - dans les modes de carte listés au bloc `signs` de `data/ui/campaign_map.json`
    (sceaux : Mécontentement ; marteaux : Richesse, Population) ;
  - d'eux-mêmes quand ils pressent : sceau ou site qui expire ce tour, rencontre en attente de
    décision, et sites de rencontre tant qu'une armée est sélectionnée (ce sont ses destinations).
- Une seule règle, `MapReadability.sign_shown(kind, mode, urgent, army_selected)`, lue par
  `ConstructionMarkers`, `IncidentMarkers` et `EncounterController`. Rendu seulement : la simulation
  et les décisions en attente ne changent pas.

## Conséquences
- La carte par défaut ne porte plus que noms, écus des lieux majeurs et armées.
- Un incident à plusieurs tours d'échéance n'est visible que par le calque ou le mode
  Mécontentement jusqu'à son dernier tour : à surveiller en partie pilote (repli possible :
  `keep_urgent` élargi ou `modes` complétés, sans code).
- `fk5_incidents_test.gd` allume le calque avant de vérifier la position du sceau.
- La frontière au repos / pleine intensité (même lot) suit le même principe : l'information
  complète vient avec la sélection, le survol ou un mode de carte.
