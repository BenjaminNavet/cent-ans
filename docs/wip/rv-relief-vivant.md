# RV — relief vivant (fin de l'effet « carte IGN »)

Demande joueur 2026-10-03 : on voit encore l'estompage IGN (petites ombres de relief), effet
« carte papier » amateur ; rendre la carte plus vivante. Mandat : tout faire, agents en parallèle.

## Diagnostic
- Relief porté par les normales (×2.1 régional, ×1.3 près, `terrain.gdshader` ~l.449), maillage plus plat :
  ombrage sans volume ni ombre portée = estompage.
- Soleil fixe à 45° d'élévation (convention estompage cartographique), ambiance plate.
- Ombres de nuages CV1 existantes mais faibles (0.18, bruit en taches), sans lien avec les nuages.
- Micro-relief haute fréquence = grain de papier.

## Lots (branche feat/rv, worktrees gp-rv-<lot>)
| Lot | Contenu | Branche | État |
|---|---|---|---|
| RV-A | Volume : normales moins exagérées, micro-relief filtré, exagération du maillage | feat/rv-a | fusionné (2aae70a36) |
| RV-B | Soleil de jeu (25-35°, chaud/froid, varie saison/tour) + perspective atmosphérique | feat/rv-b | fusionné (9f7098b78) ; soleil déjà rasant 18-21° via atmosphere.json (ADR 0156), le 45° du tscn n’était qu’un défaut |
| RV-C | Ombres de nuages crédibles, dérivant, liées aux nuages visibles | feat/rv-c | fusionné (35343960f) ; cause : amount 0 par temps clair (TB2), levée pour les cumulus |
| RV-D | Occlusion de vallée précalculée (outil tools/geo + échantillonnage shader) | feat/rv-d | fusionné (f50fb614e) ; data/map/relief_occlusion.png 3,4 Mo, `cent-ans geo relief-occlusion` |
| RV-F | Vie visible en vue régionale (fumées, reflets d'eau, vent forêts) | feat/rv-f | fusionné (ad6fd3a91) ; conflits water/campaign_life résolus, paillettes masquées sous cumulus |

ADR : 0168 (rendu du relief de campagne).

## Intégration (2026-10-03)
- Smoke OK, tb2/tb6/rv_b/zg8 OK, 3 captures en fenêtre réelle (shaders compilés).
- Retouches : panaches plus courts/discrets (smoke_alpha 0,35), paillettes limitées au chemin du soleil
  (glint_tail 0,03 : la traîne faisait des pois réguliers).
- ADR 0168 écrit.

## Restes
- Banc FPS sur machine calme (relief filtré, panaches).
- Arbres proches FC6 sans ombre de nuage / occlusion ; fleuves sans paillettes.
- Jugement joueur en jeu : douceur du relief, brume en vue régionale (~34°), panaches, vent.
