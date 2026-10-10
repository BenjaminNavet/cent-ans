# CO — colonies : moins de villes, lisibilité, onglet Bâtiments, images du Codex

Demande joueur (2026-10-10). Mandat : autonomie complète, fusion dans main et push, sans validation
intermédiaire ; ≤ 10 captures ; images en local d'abord (Z-Image Turbo, ADR 0190), payant
(OpenRouter/fal) seulement pour les échecs flagrants, enveloppe ≈ 5 $ consignée dans `docs/budget.md`.

## Décisions du joueur
- **Colonies** : au plus **5 colonies par province**, tous types confondus (2 148 → ~1 400).
  On garde la cité + les 4 places les plus notables (au moins un village par province quand il y a la place,
  diversité château/abbaye célèbre). Les villes mineures conservées deviennent des villages (viser
  nettement plus de villages que de villes). Pas de nouveau type « bourg ».
- Les colonies retirées deviennent des **hameaux décoratifs** (non sélectionnables) : `data/map/hamlets.json`.
- **Emplacements** (`building_slot_cap`) : cité 6, ville 4, château 3, abbaye 3, village 2.
- **Rééquilibrage** : remise dans les bandes (campaign_probe 120 tours × 6 graines).
- **Lisibilité** : à la sélection d'une colonie, les autres colonies de la province apparaissent en pastilles
  reliées à la cité.
- **Onglet Bâtiments** : grande illustration de la colonie à son palier (6 paliers × 5 types = 30 images,
  style occidental), puis les emplacements en cartes illustrées (image du niveau, niveau, effets), survol =
  chaîne d'amélioration. Palier = somme des niveaux de bâtiments rapportée au maximum + fortifications
  (purement visuel, calcul dans `core/`).
- **Images de bâtiments** : une image par niveau (chaque niveau est déjà une entrée `data/buildings/`) ;
  8 manquantes : bailiwick, banal_oven, corn_hall, manor_chapel, provostry, tithe_barn, toll_post, town_hall.
- **Codex** : les 76 entrées sans image (animaux, arbres, roches, oiseaux, économie, 4 personnages) en style
  herbier/bestiaire pour la nature ; revoir la pertinence des images réutilisées.

## Lots (agents Sonnet, worktrees)
| Lot | Contenu | ADR | État |
|---|---|---|---|
| CO-A | réduction des colonies, hameaux, plafonds, références, tests, équilibrage | 0291 | lancé |
| CO-B | pastilles de province à la sélection + rendu des hameaux décoratifs | — | lancé |
| CO-C | palier de colonie (core + pont) + refonte de l'onglet Bâtiments | 0292 | lancé |
| CO-D | 30 images de paliers + 8 images de bâtiments | 0293 | lancé |
| CO-E | images du Codex (76) + revue des réutilisations | 0293 | lancé |

Conventions partagées :
- Paliers : `game/assets/illustrations/settlement_tiers/<kind>_<n>.jpg`, kind ∈ city/town/castle/abbey/village, n = 1..6.
- Hameaux : `data/map/hamlets.json` = `{"description": str, "hamlets": [{"id", "name", "province", "lonlat": [lon, lat], "former_kind"}]}`,
  schéma `data/schemas/hamlets.schema.json` (écrit par CO-A).

## Prochaine étape
Relire et fusionner chaque lot à son retour ; contrôle visuel final (≤ 10 captures).
