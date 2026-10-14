# CC — carte composée façon Total War

Spécification : `docs/design/2026-10-14-carte-composee.md` ; ADR 0355.
Choix du joueur (14/10) : réinterprétation (C), Europe + Méditerranée, ≈ 400 lieux, géographie
reconnaissable mais caricaturée ; batailles au relief réel ; 0 $.

## État
- 14/10 : inventaires faits (chaîne de fabrication, références aux identifiants), spec + ADR.
- C0a fait (`docs/wip/cc-c0a-selection.md`) : périmètre `realm`, 208 provinces, 400 lieux.
- C0b fait (`docs/wip/cc-c0b-cartogramme.md`) : cartogramme par diffusion + planches.
  Itération v2 : sigma 200 km, densité de la mer 1,0 (au lieu de 150 km / 0,6) : la Méditerranée,
  l'Adriatique et l'Égée ne sont plus écrasées, l'Italie redevient une botte. Strength 1,0 écarté
  (Italie et Ibérie méconnaissables). Planches : `~/dev/cent-ans-raw/compose/v2/`.

## Prochaine étape
- **Porte C0** : le joueur choisit strength 0,5 ou 0,7 (planches v2) et valide la forme.
- Puis C1 (compilateur) avec épingles d'auteur, îles sans disques, crêtes plus lisibles.

## Points ouverts
- Taille nominale de la nouvelle carte (px, `meters_per_px`) : à fixer en C0 selon la densité visée.
