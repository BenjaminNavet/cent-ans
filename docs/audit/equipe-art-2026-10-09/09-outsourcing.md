# Outsourcing manager (art) — état des lieux (09/10, lecture seule)

## 1. État actuel
**Fournisseurs payants**
- OpenRouter (gpt-5-image-mini, gemini-3.1-flash-image, gpt-audio-mini) : portraits, icônes, illustrations, matières, voix ; ~60 $.
- fal.ai (TRELLIS 1/multi, Z-Image, flux-2/edit, bria, nano-banana-2, ElevenLabs v3) : GA3, HB, VN, VX, nuit DN. `fal_spend.jsonl` = **43,14 $** (registre DN ≈ 42,42 $). Postes : flux-2/edit 11,52 ; trellis-2 9,00 ; trellis/multi 8,06 ; trellis 7,72 ; z-image 6,84.
- Total cloud ≈ 117 $.

**Gratuits** : mflux Z-Image, Qwen-Image-Edit-2511, SF3D, Space HF TRELLIS, rembg (`docs/pipeline-assets-3d.md`, ADR 0210).

**Assets libres** : `third_party/` avec `SOURCE.md` par lot ; CC0, CC BY, CC BY-SA, OFL ; **aucune licence NC ni secret commité** ; `CREDITS.md` (576 l., lu en jeu) ; `LICENSE-ASSETS.md` (CC BY-SA 4.0) ; 2 085 glb DN hors git (ADR 0212).

## 2. Forces
- Registre détaillé, rapproché ; gardes de plafond.
- Provenance par asset (`prompt.txt`, `generation.json`, `scores.json`, `failures.jsonl`).
- Licences vérifiées une à une ; kits CC-BY douteux écartés.
- Revue qualité (~2 % d'erreurs flagrantes) ; virage vers le gratuit.

## 3. Faiblesses
1. **Règle « jamais TRELLIS 2 » violée et contredite** : 30 appels (9 $) ; `pipeline-assets-3d.md:115` dit « autorisé le 09/10 » ; `dn_batch.py` expose `--backend3d fal2` (l. 641).
2. **Règle « pas de retry local si fal échoue » inversée** : repli par défaut TRELLIS HF → SF3D → Qwen (`pipeline-assets-3d.md:307-309`), `--no-local-fallback` requis.
3. Meilleur-de-N sur modèle payant mentionné (`local-retry.md:33`) contre `pipeline-assets-3d.md:17`.
4. Enveloppes incohérentes (DN ≤ 10 $ vs ~42 $ ; plafonds 29,50 / 43,5 $) ; registre qui dérive ; `budget.py` ne lit que la dernière table.
5. **Attribution IA incomplète** : `CREDITS.md:533` « Modèles 3D : générés par scripts Blender » ; rien sur TRELLIS, Z-Image, FLUX.2, Qwen, SF3D, Gemini ; `LICENSE-ASSETS.md` ne couvre pas les glb, icônes, textures générés.
6. Licences à vérifier : SF3D (seuil de revenu, mention Stability), FLUX.2 [dev] sur fal, KayKit.
7. Provenance hors dépôt (`~/dev/cent-ans-raw`, machine unique).
8. Taux de rejet éparpillés ; prompts correctifs en négations (ignorées par Z-Image).

## 4. Améliorations
| # | Action | Impact | Effort | Coût | Dépend de |
|---|---|---|---|---|---|
| P1 | Règles du joueur dans le code : bloquer `fal2`, `--no-local-fallback` par défaut, refus multi-graines payant ; corriger la doc | Très fort | S | 0 | outils, DA |
| P2 | `CREDITS.md` + `LICENSE-ASSETS.md` : section « Modèles 3D générés » | Fort (dépôt public) | S | 0 | producer |
| P3 | Registre à plat + `cent-ans budget reconcile` | Fort | M | 0 | producer |
| P4 | Manifeste de provenance dans le paquet de modèles ; prompts versionnés | Moyen-fort | M | 0 | release |
| P5 | Tableau de bord qualité fournisseur ; arbitrer flux-2/edit vs Qwen local | Moyen | M | 0 | QA, DA |
| P6 | Audit licences SF3D/FLUX.2/KayKit + CI « pas de NC, `SOURCE.md` présent » | Moyen | S-M | 0 | CI |
| P7 | Prompts correctifs sans négation, un seul appel TRELLIS 1 | Moyen | S | ~0,3 $ | DA |
| P8 | ADR HF PRO (9 $/mois) vs recharge fal | Moyen | S | 9 $/mois | joueur |
