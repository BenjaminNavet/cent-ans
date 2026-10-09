# Environment artist — état des lieux (09/10, lecture seule, aucune image regardée)

## 1. État actuel
**Campagne — villes** : trois styles. Maquette par défaut (ADR 0158/0214, `town_maquette_layer.gd`, `dn_campaign_models.gd`, 78 glb, 21 sous-familles + port) ; réel 1:1 (`town_builder.gd`, ADR 0138/0144) ; landmarks v2 (8 villes, `docs/landmarks-v2.md`) ; landmarks v1 (7 villes, sans Orléans). Hameaux DN par famille, rétrogradation cité → bourg → village (ADR 0216) ; 86 sites réels (`map_landmarks_extra.json`).
**Campagne — nature** : arbres généralisés (ADR 0161), 40 essences, lod1 près du point visé (ADR 0221) ; 146 massifs ; lacs 762 → 1 231, zones humides 32 → 77 ; sols HB, parcellaire en shader.
**Bataille** : arbres procéduraux DA6 3 LOD ; herbe FA7 ; décor EP6 posé par Rust ; sièges `battle_siege.gd` (murs procéduraux triplanaires), toile de fond pour 7 villes ; style régional = un seul curseur colombage/Midi (`building_regions.gd`).

## 2. Forces
- Géoréférencement sérieux (EPSG:3035, OSM, ALPAGE, enceintes datées).
- Données naturelles sourcées, écarts de règles mesurés à chaque recuisson.
- 21 sous-familles culturelles ; métrique de trous gratuite (`dn_holes.py`).
- Décor de bataille à effet de jeu ; interrupteurs A/B partout.

## 3. Faiblesses (incohérences campagne / bataille)
1. **Essences** : 6 feuillus en bataille (`battle_trees.gd:38-66`) contre 40 en campagne → chênes et peupliers dans les Alpes ou en Provence.
2. **Kit DN de bataille jamais branché** (courtine, tour, porte, gravats, pont, murets, terrasses, barbacane) : murailles procédurales « plates de près » (`docs/wip/dn/audit-bataille.md`).
3. **Architecture** : 21 sous-familles en campagne contre un curseur en bataille → village rus' ou maghrébin devenu hameau normand.
4. Pas de château générique en bataille ; Orléans sans maquette ni toile de siège.
5. Forêts DN (ADR 0221) sur branche non fusionnée ; lod1 « amas de facettes » ; pommier et épicéa cassés ; décision « massif d'un seul tenant » en attente.
6. Restes HC/HB : lacs historiques, terrasses méditerranéennes, HC4, semis lent, aucun banc sans vsync.
7. 22 maquettes au dos sombre ; `farm_*`, `econ_*` non branchés ; ME6 sur volumes de repli.
8. Toute intégration dépend du paquet DN (1,1 Go hors git, ADR 0212).

## 4. Améliorations
| # | Action | Impact | Effort | Coût | Dépend de |
|---|---|---|---|---|---|
| 1 | Essences de bataille selon le biome du site (conifères, méditerranéennes) | Fort | M | 0 | tech art, bataille |
| 2 | Kit DN de siège dans `battle_siege.gd` avec repli procédural | Fort | M-L | 0 | siège, perf, ADR 0212 |
| 3 | Architecture de bataille par sous-famille DN (`subfamily_for`) | Fort hors France | M | 0 | bataille, données |
| 4 | Fusionner DN-FORET après banc sans vsync ; trancher « massif d'un seul tenant » | Moyen-fort | S | 0 | perf, joueur |
| 5 | Château générique de siège + toile de fond d'Orléans | Moyen | L | 0-0,5 $ | game design |
| 6 | HC4, puis `farm_*`/`econ_*`/`env_*` DN branchés | Moyen | M | 0 | terrain |
| 7 | Lacs historiques, terrasses méditerranéennes | Moyen | M | 0 | géo, équilibrage |
| 8 | Multi-vues des 22 maquettes au dos sombre | Moyen | S | ~1,5-2 $ | recharge fal |
| 9 | Houppiers proches propres | Moyen | M | 0 | tech art |
| 10 | Rochers GA3 et aubépine DN en bataille | Faible-moyen | S | 0 | — |

Ordre : 4 → 1 → 3 → 2, puis 6, 7, 9. Seule la ligne 8 est payante.
