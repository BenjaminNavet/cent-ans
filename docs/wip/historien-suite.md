# WIP — Historien, suite (pistes du README histoire)

Session historien, 24 septembre 2026. Source : `docs/histoire/README.md` (« Pistes pour la suite ») et audit § 7.

| Lot | Contenu | État |
|---|---|---|
| S1 | 12 personnages (audit § 7) + 14 fiches Codex (Abu al-Hasan et Valdemar IV : fiche seule, faute de faction) | fusionné — `docs/wip/s1-personnages.md` |
| S2 | Prénoms arabes andalous (`names_ar`, cul_andalusi) + ressource étain `res_tin` (Cornouailles, Devon) + fiche stannaries | fusionné — `docs/wip/s2-noms-etain.md` |
| S3 | Icônes des 7 régimes + icône `res_tin` | fusionné — `docs/wip/s3-icones-regimes.md` |

Hors lot : portraits des nouveaux personnages (clé OpenRouter après le 1er octobre).

Points ouverts :
- ~~IA : `prov_normandie_ouest` reçoit au premier tour une armée sans général~~ — résolu en P1 (`sim_campaign::frontier`, classification unique setup/IA ; plus aucune garnison scindée au premier tour). Godefroy d'Harcourt pourrait désormais démarrer en `prov_normandie_ouest` (Saint-Sauveur-le-Vicomte) si l'historien le souhaite.
- ~~L'étain ne sert à aucun bâtiment~~ — résolu sans toucher au schéma : nouveau bâtiment `bld_tin_blowing_house` (maison de fonte d'étain, tier 1, production, exige `res_tin` + rivière ; icône delapouite/furnace). Un `required_resource` à plusieurs valeurs reste possible plus tard si un bâtiment doit accepter l'une de plusieurs ressources.
