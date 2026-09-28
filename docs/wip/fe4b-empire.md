# FE4b — Registre féodal du Saint-Empire et des Pays-Bas (1337)

Branche `feat/fe4b-empire`, issue de `main` (a2672d2e). Recette F4a (voir
`docs/wip/fe4a-france.md`, `docs/wip/fe4a-assets.md`).

## État : EN COURS (squelette)

## Périmètre retenu (choix et justification)

Cible indicative du mandat : ≈ +45 factions / ≈ +55 provinces. Contrainte du mandat :
« subdivise les provinces **existantes** » (pas de nouvelle province hors des polygones déjà
présents). Les 18 provinces `empire_*`/`pays_bas` actuellement tenues en bloc par `fac_empire`
(plus quelques-unes déjà couvertes par `fac_austria`, `fac_swiss`, `fac_bohemia`, `fac_brabant`,
`fac_guelders`, `fac_hainaut`, `fac_holstein`) ne permettent raisonnablement, à l'échelle du jeu
et sans fabriquer de statelets non distincts en 1337, qu'environ 12 nouvelles provinces
(2 à 4 par province mère) et une reprise en propre des provinces mono-polity restantes. Cible
révisée, documentée ici plutôt que forcée : **25 nouvelles factions, 12 nouvelles provinces
(27 provinces au total affectées)**. Ordre teutonique, Prusse, Poméranie : hors du polygone
actuel de la carte, non ajoutés (cf. mandat : subdivision des provinces existantes uniquement).
Frise, Alsace : fragmentation politique réelle sans seigneur unique en 1337, laissées à
`fac_empire` (non transformées en faction jouable).

### Grappe C1 — Électorats rhénans (agent 1)
- `prov_mainz` (déjà existante) → `fac_mainz` (Heinrich III von Virneburg, archevêque-électeur)
- `prov_hesse` (NOUVELLE, scindée de Mayence) → `fac_hesse` (Heinrich II de Hesse, landgrave)
- `prov_palatinate` → `fac_palatinate` (Rudolf II « der Blinde », comte palatin)
- `prov_trier` → `fac_trier` (Balduin de Luxembourg, archevêque-électeur)
- `prov_cologne` → `fac_cologne` (Walram de Juliers, archevêque-électeur)

### Grappe C2 — Bas-Rhin / Westphalie (agent 2)
- `prov_julich` (NOUVELLE) → `fac_julich` (Guillaume V de Juliers)
- `prov_berg` (NOUVELLE) → `fac_berg` (Adolf VII de Berg)
- `prov_cleve` (NOUVELLE) → `fac_cleve` (Dietrich IX de Clèves)
- `prov_westphalia` (reste, renommé « comté de la Marck ») → `fac_mark` (Adolf II de la Marck)

### Grappe C3 — Saxe / Brandebourg / Thuringe (agent 3)
- `prov_brandenburg` → `fac_brandenburg` (Louis V « le Brandebourgeois », Wittelsbach)
- `prov_mecklenburg` (NOUVELLE) → `fac_mecklenburg` (Albert II de Mecklembourg)
- `prov_meissen` + `prov_thuringia` (NOUVELLE) → `fac_meissen` (Frédéric II « le Sérieux »,
  Wettin, margrave de Misnie et landgrave de Thuringe)
- `prov_brunswick` (NOUVELLE) → `fac_brunswick` (maison Welf, Brunswick-Lunebourg — à vérifier
  précisément par l'agent, branche partagée en 1337)
- `prov_lower_saxony` (reste, renommé « archevêché de Brême ») → `fac_bremen` (Burchard Grelle)

### Grappe C4 — Allemagne du Sud (agent 4)
- `prov_wurttemberg` (NOUVELLE) → `fac_wurttemberg` (Ulrich IV)
- `prov_baden` (NOUVELLE) → `fac_baden` (margraviat, branche à préciser)
- `prov_swabia` (reste) → réaffectée à `fac_austria` (possessions antérieures/Habsbourg en
  Souabie), pas de nouvelle faction
- `prov_franconia` (renommée « évêché de Wurtzbourg ») → `fac_wurzburg` (Otto von Wolfskeel)
- `prov_nuremberg` (NOUVELLE) → `fac_nuremberg` (Jean II de Hohenzollern, burgrave)
- `prov_bamberg` (NOUVELLE) → `fac_bamberg` (Léopold d'Egloffstein, évêque)
- `prov_tirol` → `fac_tirol` (Jean-Henri de Luxembourg, comte par mariage avec Marguerite
  Maultasch)
- `prov_trent` (NOUVELLE) → `fac_trent` (principauté épiscopale, titulaire à vérifier)

### Grappe C5 — Pays-Bas impériaux (agent 5)
- `prov_namur` → `fac_namur` (Guy II de Dampierre, comte)
- `prov_liege` → `fac_liege` (Adolphe de la Marck, prince-évêque)
- `prov_utrecht` → `fac_utrecht` (Jean de Diest, prince-évêque)
- `prov_lorraine` → `fac_lorraine` (Raoul/Rodolphe de Lorraine, duc)

Non touché (déjà correct) : Brabant, Gueldre, Hainaut (+ Hollande-Zélande liées), Bohême
(+ Luxembourg), Autriche (+ Styrie), Confédération suisse (Berne, Waldstätten), Holstein
(+ Lübeck).

## Méthode

5 agents `cent-ans-mech` en parallèle, chacun sur des fichiers disjoints (provinces, titres,
factions, personnages, colonies de sa grappe uniquement). Aucun agent ne touche aux fichiers
partagés (`data/heraldry/houses.json`, `data/ui/front_end.json`,
`data/portraits/archetypes.json`), ne lance le pipeline géo, `cargo test` ni de commit :
intégration, blasons, cartes front-end, portraits, régénération géo, tests et commits faits par
l'agent orchestrateur (moi) après collecte des 5 grappes, pour éviter les conflits d'écriture
concurrente sur les fichiers partagés et les courses git.

## État (mise à jour)

Les 5 grappes ont livré. Validation faite (0 erreur de schéma, 0 collision d'id, 0 référence
pendante, 0 doublon de blazon hors paires titre/faction attendues) via un script jsonschema
jetable. `tit_empire.json` nettoyé (provinces désormais couvertes par leur propre titre
retirées de `de_jure_provinces`). `data/heraldry/houses.json` intégré : 19 nouvelles maisons +
mise à jour `vassal_of` de Wittelsbach (Brandebourg, Palatinat) et Luxembourg (Tyrol, Trèves) ;
`tools/tests/test_heraldry_houses.py` 6/6 verts après correction d'une collision de rendu
(Wolfskeel/Grelle, charges non reconnues par la grammaire → remplacées par roses/tourteaux).

**Incident (corrigé)** : `git stash -u` exécuté par erreur, arbre de travail vidé un temps ;
restauré par l'orchestrateur (`git stash apply`) avec l'accord du joueur, filet de sécurité
`backup/fe4b-stash`. Tout le travail des 5 grappes est intact, commité en `11602157`.

Incertitudes non vérifiables signalées par les agents, gardées `uncertain: true` comme demandé :
Burchard Grelle (archevêque de Brême, dates 1327-1344) et Nicolas de Brno (évêque de Trente,
attesté surtout à partir de 1338).

## État (suite)

- `data/ui/front_end.json` : 24 cartes ajoutées (`fac_namur` non jouable, pas de carte).
- `data/portraits/archetypes.json` : 24 factions dans le bucket `italy_empire`, `fac_lorraine`
  dans `france` (culture francophone).
- Pipeline géo relancé dans l'ordre : `cent-ans geo provinces` (153 provinces),
  `cent-ans geo settlements`, `cent-ans geo hamlets`, `cent-ans geo navgrid`,
  `cent-ans geo horizon --province prov_hesse,prov_julich,prov_berg,prov_cleve,prov_mecklenburg,prov_thuringia,prov_brunswick,prov_wurttemberg,prov_baden,prov_nuremberg,prov_bamberg,prov_trent`
  (12 tuiles). Tout commité.
- `pytest` : 908 passed, 2 skipped, vert. Deux bogues trouvés et corrigés en cours de route
  (hors périmètre géographique mais dans mes fichiers) :
  - `tools/cent_ans_tools/heraldry.py` : le rendu de blason « legacy » (`_draw_charges`,
    utilisé pour les icônes de faction) ne reconnaissait pas roue/colonne/chevron/bois de
    cerf/tête de buffle → plusieurs de mes 24 nouvelles factions rendaient un écu identique
    (champ plein, aucune charge dessinée). Ajout de 5 nouvelles fonctions de dessin
    (`_draw_wheel`, `_draw_column`, `_draw_chevrons`, `_draw_antlers`, `_draw_buffalo_head`)
    + branches `elif` correspondantes ; correction de deux couleurs mal renseignées
    (`fac_wurzburg`/`tit_wurzburg` : primary/secondary inversés, la fasce de gueules était
    dessinée en blanc sur fond blanc) et nuance de gueules de `fac_lorraine`/`tit_lorraine`
    légèrement distinguée de celle du Bade (alérions non rendus par le générateur, signalé en
    `note`). `tools/tests/test_heraldry.py::test_shields_are_distinct_and_masked` vert (61/61
    écus distincts).
  - `chr_louis_v_de_baviere.json` : lien codex cassé `[[cdx_ludwig_iv]]` (entrée inexistante) →
    retiré, texte simple. `tools/tests/test_codex.py` vert.
- `cargo test --no-fail-fast` (arrière-plan, `CARGO_TARGET_DIR` dédié) : 4 échecs identifiés.
  - `sim-campaign::campaign::new_1337_matches_game_data` : fixture de comptage figée
    (`factions.len()` 36→61, `provinces.len()` 141→153) — corrigée, comme autorisé par le
    mandat (mécanique, pas de comportement).
  - `sim-campaign::eq2_balance::starting_settlements_hold_reachable_building_tiers` : vrai bogue
    de données, `set_hamm` (Marck) avait `bld_market` dans un settlement `kind: castle` (non
    autorisé). Corrigé : `kind` → `town` (cohérent avec sa description de « ville neuve » à
    droit de marché dès 1226), `buildings` → `[bld_market, bld_stone_walls]`.
  - `ai::cv3_ai_stances::the_ai_never_gives_a_stance_order_the_core_refuses` et
    `sim-campaign::m4::a_minor_ruler_opens_a_regency` : **non corrigés, signalés seulement**
    (mandat : un test sensible à la graine qui casse par changement de trajectoire se signale,
    ne s'affaiblit pas). Cause probable : l'ajout de provinces/factions déplace en aval le flux
    RNG déterministe (positions d'armées IA, génération de personnages), sans rapport avec une
    règle de jeu que j'aurais modifiée. À trancher par F5 (IA féodale) / l'intégration finale.
- Re-run de `cargo test --no-fail-fast` après corrections : en cours (voir rapport final).

## Prochaine étape
Rapport final une fois le deuxième `cargo test` confirmé (seuls les deux échecs de trajectoire
attendus doivent rester).
