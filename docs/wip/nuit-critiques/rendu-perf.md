# Critique rendu et performance (nuit du 22/10)

Rôle : programmeur graphique. Lecture seule de `client/src/**/*.wesl`, `client/src/**/*.rs`, `docs/wip/{vq-qualite-visuelle,ts-style-total-war,bm-perf4,vq-cperf}.md`, ADR 0404. Aucune mesure ni capture lancée (pas de banc calme disponible ; les chiffres cités viennent des docs existants).

## Constats

### 1. Le seuil 40 i/s n'a jamais été vérifié sur machine calme
- Problème : ADR 0404 exige un « banc final sur machine calme (charge < 10) avant clôture ». Quatre chantiers (VQ, TS, playtest 10-17, rivers/cground/bsoldiers) ont fusionné des coûts de rendu sans ce banc.
- Preuve : `vq-qualite-visuelle.md` (« coût non vérifié (machine chargée) », « 40 fps ultra not reached ») ; `ts-style-total-war.md` § Restes : « Perf non mesurée sur machine calme ».
- Gravité : haute (la cible produit est invérifiée, la dette s'accumule lot après lot).
- Correctif : une session dédiée, machine calme, `tools/bevy_bench/ab.sh` sur campagne d=30 (pan + descente) et `--bench-battle`, tableau p50/p95/p99 par préréglage, consigné dans un ADR. Geler les lots de rendu tant qu'il n'existe pas.
- Effort : S.

### 2. Énigme `VisibilityRange` / `RenderVisibilityRanges` non élucidée
- Problème : la table `range_to_index` de Bevy 0.20 n'est jamais purgée ; rendre les plages constantes faisait repasser le GPU de 12-14 à 28 ms. Le contournement (bandes de fondu quantifiées, `fade_maquettes`) réduit le risque sans prouver la cause.
- Preuve : `vq-cperf.md` § « Analyse de l'énigme » (65 535 emplacements, saturation en quelques secondes) ; `campaign/markers/lod.rs:325`.
- Gravité : haute (un effet de bord moteur peut ressurgir avec tout nouveau type de LOD).
- Correctif : test de non-régression qui compte `range_to_index.len()` après N minutes de caméra (assert < seuil) ; sinon patch amont ou LOD maison par distance dans le shader/`Visibility` au lieu de plages distinctes.
- Effort : M.

### 3. Ombres d'impostors en 4 cascades : limite par le remplissage
- Problème : les arbres/impostors alpha-test sont dessinés dans chaque cascade ; le banc donne 28 ms (base) contre 7,7 ms sans ombres sur le panoramique d=30. Le coût a été réduit par des lots mais reste le premier poste GPU de la campagne.
- Preuve : `bm-perf4.md` (« limité par le remplissage : ~16-20 ms », « base 28 ; sans ombres 7,7 ») ; `battle/decor/atmosphere.rs:234` (`num_cascades: 4`) ; `campaign/vegetation/shadow_cast.wesl` (223 lignes, passe maison).
- Gravité : haute.
- Correctif : cascades à 2 en « medium », distance d'ombre bornée par préréglage dans `render_quality.json` ; impostors hors cascades lointaines (ombre projetée fausse par blob/texture sol au-delà de la cascade 2) ; alpha-test remplacé par dithering sur masque pré-multiplié, ou mesh de silhouette sans discard.
- Effort : M.

### 4. `terrain.wesl` monolithique : 2401 lignes, 73 appels de bruit, 15 boucles
- Problème : un seul fragment shader cumule relief, neige, sol détaillé, parchemin (montagnes dessinées 3x3 + arbres dessinés 2x3 évalués par pixel), champs, bocage. Le nombre de variantes de chemins n'est pas borné et le coût par pixel dépend de la distance de façon peu lisible. Régression d'occupation (registres) probable, non mesurée.
- Preuve : `campaign/terrain/terrain.wesl` (`grep -c "for ("` = 15, boucles 3x3 aux l.535, 1064, 1544, 1927 ; `fbm` ligne 511) ; `ground_detail.wesl` ajouté en plus.
- Gravité : moyenne à haute.
- Correctif : découper en fonctions par palier de distance avec early-out par `footprint` (déjà passé, mais pas de coupe franche) ; précalculer les motifs parchemin (montagnes/arbres) dans une texture de cellules, un seul échantillon par pixel au lieu de 9 + 6 évaluations ; profiler avec Xcode GPU capture (hors focus : capture programmatique) pour chiffrer registres/ALU.
- Effort : L.

### 5. Pas de budget d'`overdraw` ni de tri pour la végétation billboard
- Problème : `vegetation/billboard.wesl` (489 lignes) fait de l'alpha-test sur des quads de forêt ; la nuée de plaques de bataille (`ts-style`) y ajoute. Le remplissage décide du coût (cf. 3), mais rien ne borne le nombre de couches visibles par pixel.
- Preuve : `bm-perf4.md` (« sans billboards 28,8 » donc peu d'effet isolé, mais coût caché par l'ombre) ; `ts-style-total-war.md` (T2 forêts, T8 arbres épars, T9 sous-bois).
- Gravité : moyenne.
- Correctif : alpha-to-coverage avec MSAA 4x en campagne (la caméra est déjà sans SSAO), ou dithering temporel ; compteur « pixels de végétation par image » dans le banc.
- Effort : M.

### 6. Rafale de spawns au départ de la descente caméra
- Problème : une image à 130-170 ms (`calculate_bounds`, `check_entities_needing_specialization`) lorsque pluie, neige, bannières, nuages et villes apparaissent ensemble. Perceptible en jeu (saccade à chaque entrée en campagne et chaque changement de palier).
- Preuve : `vq-cperf.md` (« plancher 130 ms », pire image 250 puis 140 ms) ; `vq-qualite-visuelle.md` (ligne cperf).
- Gravité : moyenne (pas un fps moyen mais une mauvaise première impression).
- Correctif : budget de spawn global par image étendu à tous les systèmes cités (le budget partagé existe pour les maquettes seulement), `Aabb` précalculées sur les assets (évite `calculate_bounds`), préchauffage des pipelines pendant l'écran de chargement.
- Effort : M.

### 7. Écriture de `VisibilityRange` : 3,8 ms CPU par image à l'origine
- Problème : `fade_maquettes` réécrivait 1176 plages par image ; le correctif quantifié a regagné 3,3 ms mais la boucle reste linéaire en nombre de maquettes, et le thread de rendu dépend encore du GPU (submit 3 ms, main 8 ms).
- Preuve : `vq-cperf.md` (« −3,3 ms CPU/image », « Le fil de rendu dépend encore du GPU »).
- Gravité : moyenne.
- Correctif : n'itérer que sur les entités qui changent de bande (`Changed<>` ou file de transitions triée par distance caméra), pas sur l'ensemble.
- Effort : S.

### 8. Chaîne d'éclairage bataille : SSAO conditionné à MSAA off, tonemapping et HDR non finalisés
- Problème : SSAO exige `Msaa::Off` (commentaire de `lighting.rs`), donc l'anticrénelage dépend du préréglage ; la campagne n'est pas HDR (« caméra non HDR », `cgrade`) et le grade est appliqué en post maison, ce qui limite bloom/tonemapping cohérents entre campagne et bataille. Restes notés : SSAO ultra, tonemapping, aerial pluie/neige.
- Preuve : `battle/scene/lighting.rs:28,447` ; `vq-qualite-visuelle.md` (lots blight, cgrade).
- Gravité : moyenne (qualité et cohérence plus que fps).
- Correctif : choisir un anticrénelage unique compatible SSAO (TAA ou SMAA) pour tous les préréglages ; passer la campagne en HDR avec tonemapping Bevy et retirer le grade de saison de la passe post si possible ; documenter dans un ADR.
- Effort : M.

### 9. Eau : deux piles shader distinctes (campagne `sea/river/fresh/river_fine` + bataille `water.wesl`) sans réflexion ni LOD commun
- Problème : 4 shaders d'eau campagne (~1000 lignes) et un de bataille, avec des restes connus : teinte aérienne par pixel absente, bande mouillée des lacs, palier fin sans brume. Aucune réflexion planaire/SSR : le « reflet rasant » est une approximation analytique.
- Preuve : `water/shaders/{sea 405, river 351, river_fine 215, fresh 118}.wesl`, `battle/scene/water.wesl` ; `vq-qualite-visuelle.md` (lot rivers2, restes).
- Gravité : moyenne à faible.
- Correctif : factoriser un module WESL commun (fresnel, perspective aérienne, brume) importé par les cinq ; palier de LOD explicite documenté (rivière large / fine / trait de carte) avec test d'image de référence par palier.
- Effort : M.

### 10. Végétation et relief : dettes visuelles qui coûtent en shader plutôt qu'en données
- Problème : arbres épars en vue moyenne (« taches beiges »), rochers gris avec ombres noires sur la neige, neige absente du dessus des rochers, pas de relief/haies (cground), parcelles « effet cristal ». Chacun est traité par un nouveau passage shader (`ground_detail`, `shadow_cast`, billboards), ce qui accroît le coût par pixel.
- Preuve : `ts-style-total-war.md` § Restes ; `vq-qualite-visuelle.md` (cground, cground2).
- Gravité : moyenne.
- Correctif : privilégier des données (masques bocage, carte de rochers, splat neige 8 bits) échantillonnées une fois, plutôt que du bruit procédural supplémentaire ; règle d'équipe « un nouvel effet de sol remplace un ancien ».
- Effort : M.

### 11. Skinning bataille : `skinned.wesl` 1125 lignes + compteur atomique global sur le chemin de dessin
- Problème : le chemin de soldats animés cumule skinning, livrées, usure, LOD (`battle_lod` seulement lu depuis bsoldiers). `DRAWN_SKINNED.fetch_add` à chaque dessin est cher en contention dès qu'on parallélise l'extraction. Mesure Crécy : 9,1 ms/image (110 i/s), donc marge confortable, mais mesurée avant bsoldiers.
- Preuve : `battle/units/skinned.rs:66,1160` ; `bm-perf4.md` (Crécy 9,1 ms) ; `vq-qualite-visuelle.md` (bsoldiers « coût non vérifié »).
- Gravité : faible à moyenne.
- Correctif : remplacer l'atomique par un compteur par thread agrégé en fin d'image (ou le réserver à `--features dev`) ; remesurer Crécy 20 000 soldats après bsoldiers.
- Effort : S.

### 12. Banc de perf peu intégré au flux de travail (aucune garde automatique)
- Problème : `CENT_ANS_PERF_OFF` n'est lu qu'à 5 endroits ; l'A/B est manuel (`ab.sh`) et jamais lancé en CI. Rien n'échoue si un lot dépasse p95 25 ms.
- Preuve : `grep CENT_ANS_PERF_OFF` (5 occurrences) ; `tools/bevy_bench/ab.sh`, `bench.sh` ; ADR 0404 (règle déclarative).
- Gravité : moyenne.
- Correctif : un test de rendu hors écran `--bench-battle` court (200 images) dans le smoke qui compare à un fichier de référence `p50` et alerte à +30 %, sur la machine du joueur seulement (la CI partagée est bruitée) ; chaque interrupteur d'effet visuel exposé par un réglage de `render_quality.json` et non une variable d'environnement.
- Effort : M.

## Top 3
1. Banc sur machine calme (constat 1) puis ADR de référence : tout le reste dépend de chiffres fiables.
2. Ombres d'impostors et cascades (constat 3) : premier poste GPU de la campagne (28 contre 7,7 ms).
3. Rafale de spawns + énigme `VisibilityRange` (constats 6 et 2) : saccades visibles à chaque entrée en campagne et risque moteur non maîtrisé.
