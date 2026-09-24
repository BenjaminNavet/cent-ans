# WIP — S1 personnages manquants (audit § 7)

**État : terminé.** Source : `docs/histoire/audit-2026-09-23.md` § 7 « Personnages de 1337
manquants importants ». Déjà présents (non retouchés) : Gautier de Mauny, William Montagu,
Agnès Randolph.

## Personnages créés (12) + fiches Codex

| Id | Faction | Province | Codex |
|---|---|---|---|
| chr_jeanne_de_valois_hainaut | fac_hainaut | prov_hainaut | cdx_jeanne_de_valois_hainaut (neuve) |
| chr_hugues_quieret | fac_france | prov_normandie | cdx_hugues_quieret (déjà existante, H7/H8 ; non touchée) |
| chr_nicolas_behuchet | fac_france | prov_normandie | pas de fiche dédiée : couvert par cdx_hugues_quieret (fiche jointe) |
| chr_godefroy_d_harcourt | fac_france | prov_normandie (voir liberté) | cdx_godefroy_d_harcourt (neuve) |
| chr_olivier_iv_de_clisson | fac_brittany | prov_bretagne | cdx_olivier_iv_de_clisson (neuve) |
| chr_louis_i_de_bourbon | fac_france | prov_bourbonnais | cdx_louis_i_de_bourbon (neuve) |
| chr_gaston_ii_de_foix_bearn | fac_france | prov_bearn | cdx_gaston_ii_de_foix_bearn (neuve) |
| chr_jean_i_d_armagnac | fac_france | prov_toulousain | cdx_jean_i_d_armagnac (neuve) |
| chr_humbert_ii_de_viennois | fac_empire | prov_dauphine | cdx_humbert_ii_de_viennois (neuve) |
| chr_jacques_iii_de_majorque | fac_aragon | prov_mallorca | cdx_jacques_iii_de_majorque (neuve) |
| chr_pierre_roger | fac_france | prov_normandie | cdx_pierre_roger (neuve) |
| chr_petrarque | fac_papacy | prov_comtat_venaissin | cdx_petrarque (neuve) |
| chr_jean_le_bel | fac_hainaut | prov_hainaut | cdx_jean_le_bel (déjà existante, H8 ; non touchée) |

Non créés en personnage (aucune faction adaptée : pas de Mérinides ni de Danemark) — fiche
Codex seule : `cdx_abu_al_hasan_ali`, `cdx_valdemar_iv_de_danemark`.

## Libertés notées
- Jean Ier d'Armagnac : pas de province `prov_armagnac` dédiée ; placé à `prov_toulousain`
  (domaine royal limitrophe), signalé dans sa fiche Codex (`anachronism`).
- Jacques III de Majorque : rattaché à la faction `fac_aragon` (branche cadette vassale) plutôt
  que représenté hors carte, signalé dans sa fiche Codex.
- Godefroy d'Harcourt : placé à `prov_normandie` (Haute-Normandie) au lieu de sa baronnie réelle
  de Saint-Sauveur-le-Vicomte en `prov_normandie_ouest` — ce dernier accueille dès le premier
  tour une armée sans général issue d'un surplus de garnison (l'IA classe les provinces
  frontalières différemment entre l'initialisation 1337 et le module `ai`, un écart préexistant),
  ce qui faisait échouer `governors_and_generals_are_appointed` (`core/crates/ai/tests/
  campaign_ai.rs`) dès qu'un personnage français inoccupé s'y trouvait. Signalé dans sa fiche
  personnage et sa fiche Codex plutôt que de toucher le code de l'IA (hors périmètre de la tâche).
- Jeanne de Valois : famille reliée à `chr_guillaume_i_de_hainaut` et `chr_philippe_vi` sans
  modifier ces fichiers existants (lien à sens unique).

## Incident corrigé
Écriture initiale sans lecture préalable de deux fiches Codex déjà existantes
(`cdx_hugues_quieret.json`, `cdx_jean_le_bel.json`, des lots H7/H8) : elles ont été écrasées puis
restaurées à l'identique (commit `fix: restore pre-existing codex entries clobbered by S1
writes`). La fiche `cdx_nicolas_behuchet.json` que j'avais créée en double a été supprimée : le
personnage est déjà couvert par la fiche jointe `cdx_hugues_quieret.json` (aliases « Nicolas
Béhuchet », « Béhuchet »).

## Tests
`uv run --project tools pytest -q` : 87 passed. `cd core && cargo test` : tout vert après le
déplacement de Godefroy d'Harcourt (voir « Libertés notées »).

## Prochaine étape
Lot S1 terminé. Reste (autres agents) : S2 (prénoms arabes andalous, ressource étain), S3
(icônes régimes). Portraits des 12 nouveaux personnages : après le 1er octobre (clé OpenRouter),
via `cent-ans portraits` (le rôle `prelate` a été ajouté à `ROLE_LABELS` dans
`tools/cent_ans_tools/portraits.py`).
