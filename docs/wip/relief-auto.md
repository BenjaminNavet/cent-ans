# RA — mise à jour automatique du paquet de relief (ADR 0149)

Demande du joueur (02/10) : le paquet « Cent Ans relief » doit se mettre à jour tout seul,
publication comprise (accord durable pour les Releases de `BenjaminNavet/cent-ans-relief`).

## Constat
Le paquet v2 (29/09) valait le cache local mais restait au palier 3 v5 alors que le code attend
v6 depuis RS-G (28/09) : `BAKE_VERSION` avait été monté sans `relief_pyramid.json`
(`bake_versions`), donc `geo relief-all --check` disait « complet ».

## État
| # | Contenu | État |
|---|---|---|
| 1 | `geo relief-update` (`relief_update.py`) : manifeste ← code, recuisson, `towns` + `landmarks`, paquet, Release `gh` | fait, testé |
| 2 | Marque `pyramid/package.json` + `relief-fetch --if-needed` (`needs_fetch`) | fait, testé |
| 3 | `tools/launch.sh` étape 3 : téléchargement si cache absent ou ancien (`--no-relief`) | fait |
| 4 | Test `test_manifest_bake_versions_follow_the_code` | fait |
| 5 | Recuisson v6 (palier 3, fleuves, routes), `towns`, `landmarks` | en cours |
| 6 | Paquet v3 publié, données commitées et poussées | à faire |
| 7 | ADR 0149, `docs/geo.md`, README, `rs-g-cities.md` | à faire |

## Prochaine étape
Fin de `geo relief-all` → `geo towns`, `geo landmarks`, tests (`test_landmarks_v2*`, `test_towns`,
`vh4_landmarks_test.gd`) → `geo relief-update` (empaquette v3 et publie) → commit `data:` → push.
