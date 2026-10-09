# Art producer — état des lieux (09/10, lecture seule)

## 1. État actuel
**Dépense** : ~117 $ au total dans `docs/budget.md` (16 enveloppes), alors que l'en-tête affiche encore « plafond 50 $ v1 ».
- Nuit DN (ADR 0210, enveloppe ≤ 10 $) : **≈ 42,4 $** (production 28,96 ; DN-RESTE 2,67 ; DN-FIX3 0,65 ; DN-TROUS 10,12 dont 9 $ de TRELLIS 2 abandonné ; gerbes 0,02).
- Plafond outil relevé à 29,50 $ (`DN_FAL_CAP_USD`). Solde fal **épuisé**, quota HF ZeroGPU épuisé.

**Chantiers ouverts**
- DN : 667 objets 3D + 308 illustrations, 15 erreurs flagrantes dont 13 refaites. Restes (`docs/wip/dn/trous.md`) : ~25 maquettes multi-vues (~0,07 $ pièce), 2 figures non cuites, glacier, imposteurs d'arbres, `siege_cannon_early_1340`.
- DN-forêts : attend la décision du joueur.
- Paquet de modèles (ADR 0212) : 1,1 Go hors git, sans hébergement ni sauvegarde.
- Branches non fusionnées : `dn/champs` (10 commits, ADR 0220), `feat/faction-minis`, `dn/fig-bake`, `as8c`.
- Autres restes : mflux (blasons inventés), realisme-suite L6, `ga.md` (depuis 28/09), `hc-habillage-carte.md` (cases non cochées alors que HC1 est livré).
- Copie de travail : 13 fichiers modifiés non commités, dont `.gitignore` et `data/map/*`.

**Suivi** : `docs/status.md` daté du 24/09 (« 19,20 $ sur 50 $ ») ; `docs/roadmap.md` s'arrête à V1-V6 ; 175 notes wip, 199 ADR.

## 2. Forces
- Registre ligne par ligne avec coût réel, rapproché d'OpenRouter ; journal `fal_spend.jsonl` (2 667 lignes).
- Procédé documenté (`docs/pipeline-assets-3d.md`, charte S ≤ 0,40, provenance `dn_manifest.json`, galerie).
- Voie locale gratuite fonctionnelle (`--local`).
- Revue qualité systématique ; débit très élevé à coût unitaire bas.

## 3. Faiblesses
1. Gouvernance budgétaire : enveloppes sans plafond global, DN ×4, cumuls non numériques, décimales mélangées, ligne datée du 10/10.
2. Goulot fal/HF : 3D de qualité bloquée ; repli SF3D gris.
3. **Risque de perte** : 1,1 Go de glb (~42 $) sans sauvegarde.
4. Décisions du joueur en souffrance (forêts, ADR 0212, `ga`, lot B mflux).
5. Fusions en retard face à un `main` qui bouge vite (SC).
6. Docs de suivi périmées, pas d'index des wip.
7. 9 $ de TRELLIS 2 lancés sans sonde préalable.

## 4. Améliorations
| # | Action | Impact | Effort | Coût | Dépend de |
|---|---|---|---|---|---|
| P1 | Sauvegarder les glb DN (miroir local daté, puis ADR 0212) | Évite la perte de ~670 modèles | S | 0 | joueur |
| P1 | Réconcilier le budget (synthèse, plafond global, décimales, rapprochement `fal_spend.jsonl`, `budget.py` testé) | Confiance | S | 0 | joueur, outils |
| P1 | Règle « sonde avant lot » (1-3 appels) + garde par enveloppe | Évite les pertes | S | 0 | outils |
| P2 | Une séance de décisions joueur (forêts, ADR 0212, `ga`, mflux B) | Débloque ~4 chantiers | S | 0 | joueur |
| P2 | Fusionner `dn/champs`, `fig-bake`, `as8c`, `faction-minis` | Contenu visible | M | 0 | tech, QA |
| P2 | Restes DN gratuits (imposteurs d'arbres, canon 1340, test) | Moyen | S | 0 | tech |
| P3 | Recharge fal ~3 $ pour 25 maquettes multi-vues + glacier | Dos des villes | S | ~2 $ | joueur |
| P3 | Vrais blasons dans les miniatures | Cohérence | M | 0 | DA, historien |
| P3 | Mettre à jour status/roadmap, index des wip | Lisibilité | S | 0 | lead |
| P4 | Voie locale 3D par défaut | Indépendance | L | 0 | tech art |
| P4 | Banc A/B Metal L6 | Fiabilité FPS | S | 0 | perf |
