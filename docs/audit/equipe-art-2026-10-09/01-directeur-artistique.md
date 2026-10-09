# Directeur artistique — état des lieux (09/10, lecture seule)

## 1. État actuel
- Charte : `docs/design/2026-09-25-bible-da.md` (rév. 08/10) — enluminure pour la 2D, réalisme peint pour la 3D (3 états : détaillé / généralisé / cartographique), héraldique comme pont. ADR 0004, 0050, 0065, 0135, 0158, 0210, 0211 (D1-D5, S ≤ 0,40), 0212, 0214-0222.
- Production DN (nuit 08→09/10, `docs/wip/dn/production.md`) : 667 objets 3D (`data/art/dn_manifest.json`), 308 illustrations enluminure (`data/art/dn_catalog_illus.json`), revue 15 erreurs flagrantes / 712 (13 refaites), UI : icônes encre, curseurs, 36 événements.
- Rendu : ~150 shaders (`game/shaders/`), `game/scripts/visual/` (campaign_lighting, atmosphere_library, render_quality, building_kit) ; `dn_campaign_models.gd`, `town_maquette_layer.gd`.

## 2. Forces
- Identité forte et documentée, contrôles chiffrés (S ≤ 0,40, budgets tris/textures).
- Campagne au niveau visé (relief, fleuves, toponymes enluminés, écus, HUD parchemin).
- Menu signé (lettrine azur et or).
- Bataille : livrées saturées sur décor sourd, camps lisibles.
- Provenance et coûts tracés.

## 3. Faiblesses
1. `docs/design/2026-09-23-cent-ans-design.md` l. 25 périmé (« low-poly stylisée ») vs ADR 0004/0211.
2. Pas de captures avant/après DN : 667 objets intégrés sans validation par palier de zoom (§ 7, § 11.4).
3. S ≤ 0,40 vérifié seulement sur l'image source, pas au rendu : remplissage de faction plein, champs ocre saturés, netteté qui fait halo en bataille.
4. § 10 de la bible (dette) pas à jour : DA4 (musique en couches), DA5 (icônes game-icons) ouverts ; Quaternius/Kenney encore dans `third_party/` (22 références).
5. Figurines : `dn/fig-bake` fusionnée en wip seulement ; `archer_3_jack`, `fig_sled_driver_north` non cuits ; pas de `ga3_figure.json`.
6. Trous : 27 refusés D5 (galères, cultures, arbres méditerranéens, `ship_nef`) ; UI : ornements, 5 médaillons HUD, 23 planches, curseur `arrow` ; 5 curseurs générés mais non branchés ; sceaux de diplomatie.
7. Passage d'échelle campagne inachevé (GC4, GC6-8) : raccord Seine maquette / fleuve fin, ponts, ménage 1:1.
8. Budget incohérent : DN annoncé ≤ 10 $, journal ≈ 29 $.
9. Poids : paquet modèles ~900 Mo non poussé, `game/assets/models` 1,3 Go → risque de dérive.

## 4. Améliorations
| # | Action | Impact | Effort | Coût | Dépend de |
|---|---|---|---|---|---|
| P1 | Banc de captures DA par palier et par biome, planche avant/après DN, verdict dans `docs/img/dn/` | Évite les assets « d'une autre main » | M | gratuit | QA, tech |
| P1 | Contrôle de saturation au rendu (histogramme S), atténuer faction/ocre, revoir netteté bataille | Décor vraiment sourd | S-M | gratuit | tech art |
| P1 | Finir cuisson figurines + `ga3_figure.json` | Unités variées et cohérentes | M | gratuit | anim, tech |
| P2 | Mettre à jour design l. 25, bible § 10/§ 14.9, enveloppe DN dans `budget.md` | Cohérence | S | gratuit | producer |
| P2 | Finir GC4/GC6 (fleuves, ponts, raccords) | Moins de coutures | M | gratuit | world, tech art |
| P2 | UI P1 manquante (sceaux, médaillons, ornements, curseurs, icônes DA5) | Interface homogène | M | gratuit local / ~1 $ | UI |
| P3 | Reprendre les 27 refusés en local d'abord | Méditerranée et mer crédibles | M | gratuit / < 2 $ | pipeline |
| P3 | Retirer Quaternius/Kenney après validation | Une seule main | M | gratuit | tech |
| P3 | Musique en couches (DA4), remplacer MacLeod | Ambiance | L | crédits | audio |
| P3 | Publier le paquet de modèles + SHA-256 au lancement | Install fiable | S | gratuit | release |
