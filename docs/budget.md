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
| 2026-09-25 | OpenRouter | AR1 : planches illustrées (loading) (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,05 $ | 0,69 $ |
| 2026-09-25 | OpenRouter | AR1 : planches illustrées (vignette) (3 × openai/gpt-5-image-mini) | 0,14 $ | 0,14 $ | 0,83 $ |

## Batailles épiques (25/09) — plafond propre de 20 $

| Date | Service | Objet | Coût estimé | Coût réel | Cumul batailles épiques |
|---|---|---|---|---|---|
| 2026-09-25 | Freesound (CC0) | EP4 : 27 sons libres supplémentaires (chocs acier/acier et acier/bois, impacts d'armure, cris d'effort, râles, chutes, chevaux, 3e nappe de mêlée), licence vérifiée page par page, aucun appel payant | 0,00 $ | 0,00 $ | 0,00 $ |
