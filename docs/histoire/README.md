# Histoire et savoir — ce que le jeu enseigne

Chantier de la session « historien » (23-24 septembre 2026). Conception :
`docs/design/2026-09-23-histoire-et-savoir.md` ; audit : `docs/histoire/audit-2026-09-23.md`.

## Pour le joueur

| Où | Quoi | Touche / accès |
|---|---|---|
| Partout | **Mots « découverte »** : un mot rubriqué (rouge souligné) ouvre une bulle au survol ; les liens d'une bulle en ouvrent d'autres (jusqu'à 6). Un mot déjà lu passe en brun. | survol ; clic = fiche ; clic droit = épingler ; Échap |
| Infobulles riches | **Épingler** une infobulle pour cliquer ses mots | `T` pendant qu'elle est affichée |
| Codex | 230 fiches : personnages, dynasties, lieux, guerre, société, religion, savoirs, héraldique, calendrier, vie quotidienne, table, plantes, médecine ; compteur de découvertes, recherche, « Voir dans l'encyclopédie » | `K` |
| Encyclopédie (F8) | règles et données ; bouton « Fiche historique » vers le Codex | `L` |
| Panneau de province, onglet Ville | **La Table** : régime alimentaire (7), Carême au printemps, coût d'hiver | — |
| Technologies | **Médecine** : 3e arbre (14 techs), plantes de l'herbier, notes historiques | onglet Médecine |
| Panneau de faction | **Monnaie** (forte, saine, affaiblie, fortement affaiblie), **ordre de chevalerie**, **Captifs et rançons** | clic sur le blason |
| Chronique | ≈ 60 événements historiques et pédagogiques ajoutés (Cadzand, Artevelde, Jarretière, gabelle, feu de Saint-Antoine, Oresme…) | — |

## Libertés assumées

Chaque liberté est écrite dans la fiche concernée (encadré « Liberté prise par le jeu ») ou
dans la note `historical_year` / `historical_date` de la donnée. Exemples : écorce de saule
contre la fièvre (attestée surtout au XVIIIe siècle), Jarretière fondable dès 1344 (Table
ronde de Windsor), dates en nouveau style, saisons sans le décalage julien, l'ordre de
l'Étoile brisé par la perte de la moitié de ses membres en une saison (Mauron, 1352).

## Pour enrichir le Codex

- Une fiche = `data/codex/cdx_<id>.json` (schéma `data/schemas/codex.schema.json`).
- Liens `[[cdx_id]]` ou `[[cdx_id|libellé]]` dans les fiches, les descriptions de personnages
  et de techs et le `text` des événements (pas dans les titres ni les options).
- Lien vers une fiche pas encore écrite : lister son id dans `data/codex/_todo.md`.
- Validation : `uv run --project tools pytest -q tools/tests/test_codex.py`.

## Pistes pour la suite

Lot S1 (24 septembre 2026) fait : les 14 personnages de l'audit § 7 sont couverts — 12 nouveaux
(Jeanne de Valois de Hainaut, Hugues Quiéret, Nicolas Béhuchet, Godefroy d'Harcourt, Olivier IV de
Clisson, Louis Ier de Bourbon, Gaston II de Foix-Béarn, Jean Ier d'Armagnac, Humbert II de
Viennois, Jacques III de Majorque, Pierre Roger/futur Clément VI, Pétrarque) plus une fiche Codex
seule pour Abu al-Hasan Ali et Valdemar IV Atterdag (aucune faction Mérinides ni Danemark dans
`data/factions/`). Détail : `docs/wip/s1-personnages.md`.

Reste des suggestions de l'audit non encore faites : données (prénoms arabes pour Grenade,
ressource étain), icônes propres aux régimes, portraits des nouveaux personnages (clé OpenRouter
après le 1er octobre), bouton Codex dans le bandeau (session d'interface).
