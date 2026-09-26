# Budget cloud (v1)

Plafond : **50,00 $** pour la v1. En dessous, aucune confirmation demandée.

Note (2026-09-23, M10) : la clé OpenRouter a en plus sa propre limite mensuelle (100 $, consommée
hors projet) ; elle bloquait les portraits après le premier (402). Les lots M10 sont consignés
ligne par ligne ci-dessous (estimation au jeton, coût réel `usage.cost`).

Rapprochement (2026-09-24, session 5) : compteur OpenRouter de la nouvelle clé, `total_usage`
99,4390 $ − 80,3766 $ au départ = 19,06 $ dépensés, contre 19,15 $ consignés pour la même période
(lignes du 2026-09-24) : le registre est légèrement prudent. Crédit restant ≈ 0,56 $.

| Date | Service | Objet | Coût estimé | Coût réel | Cumul |
|---|---|---|---|---|---|
| 2026-09-23 | OpenRouter | Sonde gpt-5-image-mini (16 jetons, test de la limite de clé) | 0,01 $ | 0,00 $ | 0,00 $ |
| 2026-09-23 | OpenRouter | Portraits M10 (1 × openai/gpt-5-image-mini) | 0,02 $ | 0,05 $ | 0,05 $ |
| 2026-09-24 | OpenRouter | Portraits M10 (3 × openai/gpt-5-image-mini) | 0,14 $ | 0,15 $ | 0,20 $ |
| 2026-09-24 | OpenRouter | Miniatures d'événements (3 × openai/gpt-5-image-mini) | 0,14 $ | 0,15 $ | 0,35 $ |
| 2026-09-24 | OpenRouter | Miniatures d'événements (3 × openai/gpt-5-image-mini) | 0,14 $ | 0,15 $ | 0,50 $ |
| 2026-09-24 | OpenRouter | Illustrations de l'encyclopédie (2 × openai/gpt-5-image-mini) | 0,10 $ | 0,09 $ | 0,59 $ |
| 2026-09-24 | OpenRouter | Illustrations de l'encyclopédie (29 × openai/gpt-5-image-mini) | 1,32 $ | 1,36 $ | 1,95 $ |
| 2026-09-24 | OpenRouter | Portraits M10 (84 × openai/gpt-5-image-mini) | 3,83 $ | 3,85 $ | 5,80 $ |
| 2026-09-24 | OpenRouter | Illustrations de l'encyclopédie (85 × openai/gpt-5-image-mini) | 3,87 $ | 3,92 $ | 9,72 $ |
| 2026-09-24 | OpenRouter | Miniatures d'événements (114 × openai/gpt-5-image-mini) | 5,19 $ | 5,41 $ | 15,13 $ |
| 2026-09-24 | OpenRouter | Sonde `image_config` 16:9 (1 × openai/gpt-5-image-mini, ignorée par le modèle) | 0,04 $ | 0,04 $ | 15,17 $ |
| 2026-09-24 | OpenRouter | Illustrations du Codex (84 × openai/gpt-5-image-mini) | 3,83 $ | 4,03 $ | 19,20 $ |

## Session 7 (nuit du 24/09) — enveloppe propre de 50 $

| Date | Service | Objet | Coût estimé | Coût réel | Cumul session 7 |
|---|---|---|---|---|---|
| 2026-09-25 | OpenRouter | UR1 : illustrations des 14 nouveaux types d'unités (14 × openai/gpt-5-image-mini, script hors registre automatique car le parseur de `budget.py` ne lit pas la table de session 7) | 0,64 $ | 0,64 $ | 0,64 $ |
| 2026-09-25 | OpenAI | VO1 : voix (294 clips gpt-4o-mini-tts : 120 répliques, 21 interventions du conseiller, 153 phrases et cris de discours ; `--dry-run` de `tools/cent_ans_tools/voice_tts.py`, plafond du lot 3 $). Sonde d'un clip refusée : HTTP 401, clé `OPENAI_API_KEY` invalide ; rien généré, rien facturé | 0,37 $ | 0,00 $ | 0,64 $ |
| 2026-09-25 | OpenRouter | VO1 : voix synthétiques, 294 clips openai/gpt-audio-mini (120 répliques, 21 interventions du conseiller, 153 phrases et cris de discours), contrôle mot pour mot (transcription et durée) avec reprises ; coût réel = somme de usage.cost des passes (0,194 + 0,027 + 0,006 + 0,007 + sondes 0,004) | 0,36 $ | 0,24 $ | 0,88 $ |
| 2026-09-25 | OpenRouter | VO1 : reprise de 2 interventions du conseiller (adv_start_france, adv_first_siege) à transcription strictement conforme, 1 essai chacune (0,0016 $) | 0,01 $ | 0,00 $ | 0,88 $ |
| 2026-09-25 | OpenRouter | AR1 : planches illustrées (loading) (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,05 $ | 0,93 $ |
| 2026-09-25 | OpenRouter | AR1 : planches illustrées (vignette) (3 × openai/gpt-5-image-mini) | 0,14 $ | 0,14 $ | 1,07 $ |

## Batailles épiques (25/09) — plafond propre de 20 $

| Date | Service | Objet | Coût estimé | Coût réel | Cumul batailles épiques |
|---|---|---|---|---|---|
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « picardy_plains » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,04 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « channel_coast » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,08 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « pyrenees » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,12 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « alps » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,16 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « massif_central » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,05 $ | 0,21 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « wooded_hills » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,25 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « norman_bocage » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,29 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « loire_valley » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,33 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « gascony_hills » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,37 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « flanders_flat » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,41 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « moorland_hills » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,45 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « mediterranean_hills » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,49 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « winter_lowlands » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,53 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « flanders_flat » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,57 $ |
| 2026-09-25 | Freesound (CC0) | EP4 : 27 sons libres supplémentaires (chocs acier/acier et acier/bois, impacts d'armure, cris d'effort, râles, chutes, chevaux, 3e nappe de mêlée), licence vérifiée page par page, aucun appel payant | 0,00 $ | 0,00 $ | 0,57 $ |

## Direction artistique (25/09) — plafond propre de 15 $

| Date | Service | Objet | Coût estimé | Coût réel | Cumul DA |
|---|---|---|---|---|---|
| 2026-09-25 | OpenRouter | DA2 : portraits vivants (archétypes et variantes âgées) (6 × openai/gpt-5-image-mini) | 0,28 $ | 0,27 $ | 0,27 $ |
| 2026-09-25 | OpenRouter | DA : planche de style, bouton de fin de tour (cloche) et planche d'icônes à l'encre (2 × openai/gpt-5-image-mini) | 0,09 $ | 0,09 $ | 0,36 $ |
| 2026-09-26 | OpenRouter | DA2 : portraits vivants (archétypes et variantes âgées) (141 × openai/gpt-5-image-mini) | 6,47 $ | 6,33 $ | 6,69 $ |
