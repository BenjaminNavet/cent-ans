# WIP — B9a : fiches Codex, sujets 1 à 9 (audit 2026-09-25 § 10)

Branche : `b9a-fiches` (partie de `integration/historien`).

## État

| # | Fiche | État |
|---|---|---|
| 1 | `cdx_pietro_barbavera` | écrite (alias « Barbavera » retiré de `cdx_galee`) |
| 2 | `cdx_charles_de_la_cerda` | écrite (alias retiré de `cdx_winchelsea`, lien dans `evt_winchelsea`) |
| 9 | `cdx_baudouin_de_luxembourg` | écrite (entité `prov_trier`, lien dans sa description) |
| 3 | `cdx_carrare` | écrite (ligue, paix de Venise ; liens fac_venice, fac_verona, chr_mastino) |
| 4 | `cdx_mastino_ii_della_scala` | écrite (entité chr_mastino_ii_della_scala) |
| 5 | `cdx_azzone_visconti` | écrite (entité chr_azzone_visconti ; lien fac_milan) |
| 6 | `cdx_taddeo_pepoli` | écrite (entité prov_bologna) |
| 7 | `cdx_savoie_achaie` | écrite (entité prov_piemont ; description corrigée : Suse au comte) |
| 8 | `cdx_conquete_de_la_sardaigne` | écrite (entité prov_sardegna) |
| — | Vérification Breteuil / Romorantin (fiches jeu) | faite : exacts ; cdx_beffroi et cdx_jeu_incendies précisés |
| — | Rapport audit § 10 | fait (section « Rédigées par le lot B9a ») |

## Prochaine étape

Lot terminé : validateur vert (397 fiches), `pytest tools/tests/test_codex.py` vert. Reste : fusion dans `integration/historien` (conflit probable avec B9b dans l'audit § 10, à résoudre à la main).

Pas de build (disque plein) : seul le validateur Python.
