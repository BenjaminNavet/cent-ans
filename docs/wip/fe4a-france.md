# FE4a — Registre féodal de la France (1337)

Branche `feat/fe4a-france`, issue de `main` (dfe88244, qui contient FE0 1dcd6821).

## État

- 9 nouvelles provinces créées (`data/provinces/`) : Alençon, Évreux, Albret, Charolais,
  Penthièvre, Blois, Foix, Armagnac, Valois. Graines/poids ajoutés pour `cent-ans geo provinces`.
  Colonies déjà présentes (`set_alencon`, `set_evreux`, `set_nerac`, `set_blois`) déplacées vers
  les nouveaux fichiers de colonies ; 5 nouvelles colonies capitales créées (Charolles, Lamballe,
  Foix, Lectoure, Crépy-en-Valois). `prov_bourbonnais` et `prov_bearn` existaient déjà : leur
  `owner` passe de `fac_france` à leurs nouvelles factions (`fac_bourbon`, `fac_foix_bearn`).
- 11 nouveaux titres (`data/titles/`) : `tit_alencon`, `tit_evreux`, `tit_albret`,
  `tit_charolais`, `tit_penthievre`, `tit_bourbon`, `tit_blois`, `tit_foix`, `tit_bearn`,
  `tit_armagnac`, `tit_valois`.
- Fusion `tit_gascogne` → `tit_guyenne` (un seul duché d'Aquitaine/Guyenne avec deux provinces
  propres), `tit_gascogne.json` supprimé.
- Corrections de rang : `tit_ponthieu` et `tit_angoumois` (duchy → county), `tit_montpellier`
  (duchy → county).
- `tit_france.de_jure_provinces` : retrait de `prov_bourbonnais` et `prov_bearn` (désormais
  provinces propres de titres séparés).
- 7 nouvelles factions jouables (`data/factions/`) : `fac_alencon`, `fac_bourbon`, `fac_blois`,
  `fac_foix_bearn`, `fac_armagnac`, `fac_penthievre`, `fac_albret`.
- Personnages : 4 créés (`chr_guy_i_de_chatillon`, `chr_bernard_ezi_d_albret`,
  `chr_pierre_i_de_bourbon`, `chr_gaston_iii_de_foix_bearn`) ; 6 modifiés (faction/role mis à
  jour pour Charles d'Alençon, Louis de Bourbon, Gaston II de Foix-Béarn, Jean Ier d'Armagnac,
  Jeanne de Penthièvre, Charles de Blois).
- Validation manuelle des schémas (`jsonschema`, script jetable) : provinces, titres, factions,
  personnages, colonies modifiés/créés — tout passe.
- Pipeline géographique (`cent-ans geo provinces`) : **pas encore relancé** à ce stade du wip
  (voir prochaine étape).

## État final

1. `cent-ans geo provinces` et `cent-ans geo settlements` relancés : 141 provinces, aperçus et
   graphe de colonies régénérés et commités.
2. `pytest tools/tests/test_feudal_titles.py` : 6/6 verts.
3. `cargo test -p data-model` : 34 + 11 verts (le second lot avait échoué deux fois — voir
   « Bogues trouvés et corrigés » — corrigé puis vert).
4. `cargo test -p sim-campaign --test feudal_deductions --test campaign` : `feudal_deductions`
   3 verts + 4 `#[ignore]` (F1, sans rapport avec ce lot) ; `campaign` 19/20 verts, un échec
   restant signalé ci-dessous (non corrigé, hors mandat).
5. Commit final `FE4a: registre féodal de la France (9 provinces, 11 titres, 7 factions)`.

## Bogues trouvés et corrigés en cours de route

- `tit_bourbon` : objectif `hold_crown` écrit avec un champ `faction` au lieu de `title`
  (l'énumération Rust `TitleObjectiveCondition::HoldCrown` attend `{ title }`, alors que le
  schéma JSON accepte les deux champs sans distinction par `kind` — schéma trop permissif,
  signalé plus bas). Corrigé : `"title": "tit_france"`.
- `chr_bernard_ezi_d_albret` : trait `trait_pragmatic` inexistant dans `data/traits/`. Remplacé
  par `trait_cunning`.
- Trois fixations de test Rust qui figeaient un décompte (province/faction/armée), toutes mises
  à jour comme autorisé par le mandat :
  - `campaign.rs::new_1337_matches_game_data` : `factions.len()` 29 → 36 (+7 nouvelles
    factions), `provinces.len()` 132 → 141 (+9 nouvelles provinces), `armies().len()` 28 → 35
    (une armée principale par faction).
  - `campaign.rs::france_income_is_positive_and_in_target_range` : `summary.provinces_count`
    27 → 26 (`prov_bourbonnais` et `prov_bearn` quittent `fac_france` pour `fac_bourbon` et
    `fac_foix_bearn` ; `prov_valois`, nouvelle, reste à la couronne : 27 − 2 + 1 = 26).

## Échec de test non corrigé (signalé, pas de code de règles touché)

`campaign.rs::succession_follows_heir_then_house_then_none` échoue désormais : après la mort de
Philippe VI puis de Jean de Normandie, le test attend que « l'aîné des Valois vivants » hérite de
la couronne, et vérifie `house == "Valois"`. Il obtient `"Quiéret"`. Cause : avant ce lot,
Charles II d'Alençon (maison Valois, `faction: fac_france`) faisait partie du vivier de
candidats de la faction France ; ce lot le transfère à sa propre faction (`fac_alencon`), comme
demandé par la spec (chaque grand vassal doit avoir sa propre faction jouable). L'algorithme de
succession actuel (`characters.rs::succeed`, recherche « maison » **au sein de la faction**) n'a
donc plus de candidat Valois dans `fac_france` et retombe sur un personnage de la maison
Quiéret. C'est un effet de bord attendu de la restructuration féodale : la vraie correction
revient à F1/F3 (succession par titres, pas par recherche de maison intra-faction) — voir
`docs/superpowers/plans/2026-09-28-feodalite.md` § F3 (« Héritage »). Je n'ai pas touché à
`characters.rs` (code de règles, hors mandat F4a) ; je n'ai pas non plus modifié le test, car ce
n'est pas un simple décompte figé mais une assertion de comportement.

## Point de schéma à examiner (hors mandat, signalé pour F0/F3)

`data/schemas/title.schema.json` (`objectives[].condition`) accepte `title`, `provinces` et
`faction` sans distinction par `kind`, alors que l'énumération Rust
(`TitleObjectiveCondition`) impose des champs précis par variante (`hold_title`/`hold_crown` →
`title` ; `hold_provinces` → `provinces` ; `be_liege_of` → `faction` ; `be_independent` → aucun).
Le schéma JSON n'aurait pas détecté mon erreur initiale sur `tit_bourbon` (voir plus haut) ; seul
`cargo test` l'a trouvée. Une validation JSON Schema par `if`/`then` sur `kind` rendrait ce genre
d'erreur détectable avant la compilation Rust — je ne l'ai pas fait ici (fichier partagé F0, hors
périmètre de ce lot de données).

## Relecture historienne (voir aussi le rapport final dans le message de fin de lot)

Points vérifiés et incertitudes assumées :
- Alençon : Charles II, comte depuis le 16/12/1325, frère de Philippe VI. Mort à Crécy en 1346 ;
  aucun héritier direct modélisé (Charles III naît en 1337, hors scope de ce lot).
- Évreux : déjà couvert par le titre `tit_evreux`, tenu par `fac_navarre` (Philippe III
  d'Évreux) — double allégeance Navarre/France, comme la Guyenne pour l'Angleterre.
- Albret : rang réel = simple seigneurie, approximé en `county` par contrainte de schéma
  (kingdom/duchy/county seulement). Suzeraineté envers la Guyenne anglaise incertaine
  (`uncertain: true`) : les sires d'Albret ont pu aussi rendre hommage direct au roi de France
  pour certaines terres ; simplifié ici à un seul lien (Guyenne).
- Charolais : aucune nouvelle faction — comté tenu en propre par les ducs de Bourgogne depuis
  1237, pas de vassal distinct en 1337.
- Penthièvre : Jeanne de Penthièvre, comtesse suo jure depuis 1331 ; mariage avec Charles de
  Blois le 04/06/1337, point de départ direct de la guerre de Succession de Bretagne (F3).
- Bourbon : duché-pairie depuis septembre 1327 (rang corrigé en conséquence, déjà correct dans
  les données existantes de `chr_louis_i_de_bourbon`).
- Blois : Guy Ier de Châtillon, comte depuis 1307, père de Charles de Blois.
- Foix-Béarn : Gaston II, comte de Foix (vassal du roi) et vicomte souverain de Béarn (alleu,
  sans hommage) — double allégeance modélisée comme la Guyenne. Rang de Béarn approximé en
  `county` (schéma). Héritier Gaston III (Fébus), 6 ans en 1337, `status: minor`.
- Armagnac : Jean Ier, comte depuis 1319, lieutenant royal en Languedoc, rival de Foix-Béarn.
- Valois : réuni au domaine royal à l'avènement de Philippe VI (1328) ; pas de vassal distinct,
  tenu directement par `fac_france`.

## Points ouverts (non traités dans ce lot)

- Population des provinces subdivisées : les nouvelles provinces reçoivent une population
  estimée indépendamment, **sans** retrancher les effectifs correspondants de la province mère
  (Maine, Normandie, Agenais, Orléanais, Toulousain, Bourgogne, Bretagne, Île-de-France/Picardie).
  Léger surcomptage du total français, acceptable pour ce lot mécanique ; à corriger dans une
  passe d'équilibrage démographique si besoin.
- Héritiers non modélisés (faute de source claire à ce budget) : Alençon (Charles III, né 1337),
  Blois (au-delà de Charles), Armagnac, Albret. `heir` reste absent sur ces factions (champ
  optionnel du schéma).
- `fac_navarre` reste `playable: false` : ses propres objectifs de victoire (mentionnés à titre
  d'exemple dans la spec § 4.8, « Navarre recouvre ses terres normandes ») ne sont pas traités
  ici — Navarre est un royaume à part, probablement du ressort de F4d (Ibérie), pas de la
  France proprement dite.
- `tit_normandie` / `tit_normandie_ouest` restent deux titres `duchy` distincts pour un seul
  duché historique (même souci que Guyenne/Gascogne avant fusion), mais tous deux tenus par
  `fac_france` : pas d'impact sur la suzeraineté (le sommet de l'arbre est déjà la couronne),
  donc non corrigé dans ce lot pour limiter la portée. Signalé pour une passe de nettoyage
  future.
- Neighbours des nouvelles provinces : laissés à `[]` dans les fichiers JSON (non utilisés par
  le jeu — `movement_graph.rs` lit `provinces.geojson`, régénéré par le pipeline géo, qui est la
  source faisant foi).
