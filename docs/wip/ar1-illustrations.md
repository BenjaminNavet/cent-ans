# WIP — AR1 : habillage illustré (chargements, vignettes d'événements, fins)

Branche : `worktree-agent-aaa4cf9bc2224eea2`.

## Principe

- Données : `data/ui/illustrations.json` (schéma `data/schemas/illustrations.schema.json`) :
  `loading.screens` (contexte battle/siege/naval/campaign, titre, citation de chroniqueur),
  `vignettes` (genres `EventKind` du cœur, ordre = priorité), `endings` (4 issues).
- Pipeline : `tools/cent_ans_tools/art_plates.py`, CLI `cent-ans assets art-plates
  [--recrop] [--generate --dry-run --envelope]` ; enluminures du domaine public (Wikimedia
  Commons, cache local `tools/.cache/commons/`, ignoré par git), recadrage déclaré dans les
  données ; génération OpenRouter (gpt-5-image-mini, comme UR1) seulement pour les manques,
  jamais deux fois un fichier existant.
- Images : `game/assets/art/{loading,vignettes,endings}/*.jpg` (≈ 6 Mo, 31 fichiers).
- Godot : `ArtPlates` (lecteur), `BattleLoadingCard` (bataille/siège/naval, ≥ 3 s, ouvert par
  `campaign_map._on_battle_fight` et `naval_campaign._on_fight`), `LoadingScreen` (campagne :
  une fois sur deux une enluminure AR1 et sa citation), `SeasonReport` (vignette du genre le plus
  prioritaire), `ChronicleWindow` (repli sur la vignette du genre), `BattleResultScreen`
  (bannière + fond plein écran), `VictoryController.show_ending` (fin plein écran).
- Captures : `--stage=loading_battle|loading_siege|loading_naval|ending_victory|ending_defeat|report_vignette`.

## État

| Étape | État |
|---|---|
| Recherche domaine public (Froissart BnF fr. 2643, BL Royal 18 E I, Vigiles de Charles VII…) | fait |
| Données, schéma, outil, 27 planches Commons | fait |
| Génération des 4 manques (Avignon, famine, trésor, commerce) : 0,19 $ réels | fait |
| Intégration Godot, import, smoke (hors échec musique préexistant de main) | fait |
| Tests pytest (`tools/tests/test_art_plates.py`), CREDITS.md | fait |
| Captures `docs/audit/captures/ar1/` (siège, naval, fin, rapport) | fait |

## Prochaine étape

Terminé. Suites : aligner cadres et palette sur UI1 (`ui_illumination.py`, pas encore dans main) ; capture de la fin de bataille en situation.
