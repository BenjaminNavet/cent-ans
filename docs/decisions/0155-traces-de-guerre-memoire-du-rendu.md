# ADR 0155 — Traces de guerre : mémoire du rendu, masque de terroir à son échelle

Date : 2026-10-02. Lot TB4 (`docs/design/2026-10-02-campagne-tob.md` § 3). Numéro à revoir à la
fusion si un autre lot TB a pris 0155 (0154 est la rotation musicale).

## Contexte

TB4 montre sur la carte les suites de la guerre et des fléaux : brûlis, peste, champs de bataille
marqués quelques tours, engins de siège du camp. Le lot est du rendu seul (`core/` intact). Trois
points demandaient un arbitrage.

1. **Champs de bataille.** Le pont donne les batailles du dernier tour (`get_events`,
   `get_pending_events` : `kind = battle`, `province`, `army`), sans coordonnées ni historique.
   Or la marque doit durer plusieurs tours, « à l'endroit de la bataille ».
2. **Brûlis invisibles.** Le déplacer après la carte de couleur (SS2) et les matières (HB3) ne
   suffisait pas : les mesures restaient identiques avec et sans dévastation. Cause : `TerroirMask`
   peint un masque carré à la même échelle sur les deux axes (1024 / plus grand côté de la carte),
   mais `terrain.gdshader` le lisait avec `uv = p / map_size`. Depuis la carte 7168 × 6144 (OM),
   tout le masque (finages, vignes, pâtures, brûlis) était décalé vers le nord d'un sixième de
   l'ordonnée (≈ 457 px à Paris).
3. **Peste.** FK4 joue déjà une scène de peste (convoi, procession, bûcher) dans le réservoir de
   figurines ; les fosses et les portes marquées sont des éléments fixes.

## Décision

1. **Le rendu garde la mémoire des champs de bataille** (`WarScars`, `game/scripts/map/`). À
   chaque tour il lit les événements `battle`, place la marque à la position de l'armée de
   l'événement (marqueur d'armée, sinon `get_army().position`), à défaut au centre de la province,
   et la vieillit avec `get_turn`. Une marque par province ; une nouvelle bataille la rajeunit. La
   durée et les étapes (corbeaux, débris, tertre) sont dans `data/ui/war_scars.json`
   (`battlefield.turns`, `crow_turns`, `debris_turns`), jamais dans le code.
2. **Le masque de terroir est lu à l'échelle où il est peint** (`p / max(map_size)`), et le brûlis
   est appliqué après `sg_apply` et `hb_apply`. La correction du masque touche toute la carte : les
   finages reviennent autour de leur colonie.
3. **Fosses, charrette arrêtée et portes marquées sont posées hors réservoir** par `WarScars`, à
   partir des scènes `plague` déjà résolues par `FolkScenes` (`edge_frame`). Fosses et charrette
   sont grossies en proportion de la distance de la caméra (`plague.scale_per_distance`), comme les
   armées : à l'échelle 1:1 des figurants FK elles font moins d'un pixel aux distances de jeu.
   Les portes restent à l'échelle réelle, sur les façades du plan de la ville 1:1 ordinaire
   (`TownLayer.plan_of`) ; pas de portes dans les villes emblématiques, où la caméra ne descend
   pas assez bas pour lire une croix. Aucun pictogramme.
5. **Brûlis par parcelles** : `hb_apply` rend la parcelle du pixel et `terroir_burn` donne à la
   parcelle entière un état (carbonisée, roussie, intacte) tiré contre la dévastation lue en son
   centre ; au loin, la moyenne des trois.
4. **Engins de siège** : enfants des figurines de l'armée assiégeante (même repère, même échelle,
   même visibilité que le camp), lus dans `get_assault_odds(armée).engines`. Trois stades :
   charpente et tas de bois, maquette sous échafaudage (`siege.almost_ready_turns`), engin prêt.
   Maquettes du kit de bataille en lecture seule (`assets/models/siege/*_lod.glb`).

## Conséquences

- Une partie rechargée ne retrouve que les batailles du dernier tour : les marques plus anciennes
  sont perdues. Pour les garder, il faudrait que le cœur expose un historique des batailles
  (tour, position) ; rien n'est changé dans `core/` ici.
- La position est celle de l'armée au moment où l'événement est lu (fin de tour), pas le point
  exact du combat ; une armée détruite retombe sur le centre de sa province.
- Les batailles d'une province sous le brouillard de guerre sont mémorisées mais cachées tant que
  la province n'est pas vue.
- Les finages reviennent autour de leur colonie sur toute la carte ; l'effet sur les mesures
  `--stats` de TB1 n'est pas chiffré (ces mesures ne sont répétables qu'avec la sonde
  `hb_debug=9`, voir `docs/wip/tb4.md`).
- Portes marquées seulement dans une ville 1:1 ordinaire chargée (vue rapprochée) ; ni villes
  emblématiques ni villages sans plan.
