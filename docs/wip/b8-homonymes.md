# WIP — B8 Auto-lien sans homonymes

Branche : `feat/b8-homonymes` (worktree agent). Contexte : `docs/wip/bulles-partout.md`, vague 2.

## État
- [x] Alias le plus long prioritaire (tri déjà présent dans `CodexStore.alias_regex`) ; limites
  de mot : le tiret entre deux mots les soude (« Saint-Omer » ne lie pas « Omer »,
  « Poitiers-sur-… » ne lie pas « Poitiers »), l'apostrophe sépare (« d'Artois » lie « Artois »).
- [x] `exclude_contexts` (liste d'expressions par fiche) : schéma, `CodexStore.is_excluded`,
  `CodexText._link_segment`, validateur (chaque expression contient le titre ou un alias) + tests.
- [x] Échappement `[[!texte]]` : texte simple jamais auto-lié. `CodexText.format` l'enveloppe
  dans `[lang=fr]…[/lang]` (balise neutre que l'auto-lien saute, même si le BBCode repasse par
  `format`) ; `CodexText.plain` rend le texte ; le validateur ne le prend pas pour un lien
  (et signale `[[!]]` vide).
- [x] Audit `tools/cent_ans_tools/codex_homonyms.py` + `tools/tests/test_codex_homonyms.py`.
- [x] Corrections (ci-dessous).
- [ ] Smoke `codex_bubbles` : cas B8.

## Audit
Lancer : `uv run --project tools python -m cent_ans_tools.codex_homonyms`.

Le script rejoue l'auto-lien du jeu (même regex, exclusions, échappements, première occurrence
par fiche) sur les textes affichés de `unit_types`, `buildings`, `technologies`, `resources`,
`edicts`, `events`, `characters`, `battle_orders`, `diets`, `chivalric_orders`, `traits`,
`skills`, `factions`, `religions`, `naval` et du Codex (`summary`, `body`, `gameplay`,
`anachronism` ; hors `sources`). Il signale un auto-lien dont l'alias (capitalisé) est pris dans :
- « Prénom de Alias » (prénom de `data/names` ou des personnages, ou mot capitalisé hors début
  de phrase) ;
- « titre de Alias » (comte, duc, bâtard, héritière…) quand la fiche n'est ni un lieu ni une
  dynastie (« duc de Bourgogne » peut lier l'État bourguignon, pas « comte de Poitiers » la bataille) ;
- « saint / Notre-Dame Alias », « Alias II » (numéro de règne).

Premier passage : 474 signalements (bruit : noms communs, sources) → heuristique affinée : 48.

## Faux liens corrigés (12 occurrences, 6 fiches, via `exclude_contexts`)
| Fiche | Contexte exclu | Où |
|---|---|---|
| `cdx_poitiers` (bataille) | Louis de Poitiers (comte de Valentinois) | cdx_campagne_de_gascogne_1345 |
| `cdx_orleans` (siège) | Philippe d'Orléans (fils de Philippe VI) | chr_raoul_de_brienne, cdx_style_de_paques |
| `cdx_orleans` | ducs d'Orléans | cdx_brisures |
| `cdx_flandre_laine` (économie) | bâtard de Flandre (Guy) | evt_cadzand ×3 |
| `cdx_flandre_laine` | héritière de Flandre (Marguerite) | cdx_philippe_le_hardi |
| `cdx_cogue` (alias « Christophe », le navire) | Christophe II de Danemark | evt_valdemar_iv, cdx_valdemar_iv_de_danemark |
| `cdx_gabelle` (alias « Salins ») | Guigone de Salins | cdx_hotel_dieu |
| `cdx_bois` | Geoffroy du Bois, « Bois ton sang » | cdx_combat_des_trente |

Exclusions préventives ajoutées aux mêmes fiches (contextes présents ailleurs dans les données
ou probables) : comte(s)/Alphonse/Aymar/université/Parlement de Poitiers, « celle de Poitiers »
(université, cdx_universite_paris) ; Louis Ier / Charles Ier / bâtard / partisans / branches /
maison / meurtre / porc-épic / duché / duchesse / Valentine d'Orléans ; Guy / Marguerite (III) /
Robert de Flandre ; Christophe Ier/III.

## Signalements restants jugés corrects (36)
Liens vers un territoire ou une maison pour une personne qui en porte le nom et le tient
(« Eudes IV de Bourgogne », « Jeanne de Navarre », « Isabelle de Valois », « Louis de Guyenne »,
« Alphonse de Castille »), lieux dans un nom composé (« Tour de Londres », « Notre-Dame de
Paris », « clos des Galées de Rouen »), « Olivier V de Clisson » (c'est bien le connétable),
« La Pucelle d'Orléans » (lié au siège : pertinent), faux positifs de l'heuristique
(« la France de Calais à Bordeaux »).

## Limites
- Les corps et résumés du Codex ne sont pas auto-liés en jeu (`format` sans `auto_link`) :
  audités quand même, pour les textes recopiés ailleurs.
- Une expression exclue qui chevauche une balise BBCode (« Louis de [b]Poitiers[/b] ») n'est
  pas reconnue.
- Heuristique du script : un nom en début de phrase dont le premier mot n'est pas un prénom
  connu n'est pas signalé.

## Prochaine étape
Smoke `codex_bubbles` (cas B8), puis validateur, pytest complet, rapport.
