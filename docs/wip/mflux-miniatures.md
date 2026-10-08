# Miniatures locales mflux (ADR 0190)

État : backend `--local` en place, testé (`tools/tests/test_local_art.py`). Essai `fac_alania`
correct (style), blasons non respectés.

Prochaine étape : quand la machine est calme (charge < 4, disque > 20 Go libres), lancer
`uv run --project tools cent-ans assets illustrations --local --category factions`
(148 miniatures, environ 2 h 30) puis contrôler un échantillon visuellement.

Points ouverts : blasons (image de référence ou incrustation), cadre parfois dessiné.
