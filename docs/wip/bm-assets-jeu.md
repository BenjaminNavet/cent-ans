# Assets de jeu Bevy — à reprendre plus tard

État (15/10) : recensement fait, **aucune adoption** ; le joueur veut y revenir après la parité visuelle.

- Recensement complet : `docs/research/bevy-assets-jeu.md` (tableaux par catégorie, top 10, versions crates.io du 15/10).
- Pendant de la productivité (adoptée/essayée dans les lots devtools1/devtools2/dataassets) : `docs/research/bevy-outils-productivite.md`.

## Candidats retenus (à réévaluer à la reprise ; vérifier d'abord la compatibilité avec la version de Bevy du client)
| Crate | Usage chez nous | À vérifier |
|---|---|---|
| bevy_kira_audio | playlists, ambiances, fondus | notre `audio/` maison couvre déjà ; gain réel ? |
| bevy_enhanced_input | raccourcis par contexte | doublon avec settings4/bkeys |
| bevy_hanabi | particules GPU (feu, fumée, poussière) | port 0.20 absent au 15/10 |
| bevy_nested_tooltips / pyri_tooltip | infobulles imbriquées (IB, style BG3) | pyri en 0.19 |
| bevy_polyline, bevy_mod_outline | frontières/routes, contour de sélection | ports 0.20 |
| bevy_rts_camera | caméra de stratégie | comparer à `campaign/camera` |
| bevy_fluent | localisation | plus tard |
| bevy_transform_interpolation | fluidité des armées | port 0.20 |

Constat : rien de mûr pour terrain LOD/clipmap, végétation instanciée, animation par texture de sommets, foules skinnées, nuages/météo → le code maison reste justifié. Navigation/physique/sauvegarde : ignorés (règles dans `core/`, sauvegarde serde).

## Reprise
1. Relire les deux notes de recherche, rafraîchir versions (crates.io API) contre la version Bevy du client.
2. Un lot « spike » par candidat retenu, verdict adopt/skip, ADR si adoption.
