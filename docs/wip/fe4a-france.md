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

## Prochaine étape

1. Lancer `uv run --project tools cent-ans geo provinces`, vérifier l'aperçu
   (`docs/img/provinces-preview.png`), commiter `data/map/provinces.geojson`,
   `data/map/province_ids.png` et l'aperçu ensemble.
2. `uv run --project tools pytest tools/tests/test_feudal_titles.py`.
3. `cd core && CARGO_TARGET_DIR=$PWD/target cargo test -p data-model` puis
   `cargo test -p sim-campaign --test feudal_deductions --test campaign`.
4. Relecture historienne (dates, détenteurs, rangs, suzerainetés) — voir section ci-dessous.
5. Commit final `FE4a: …`.

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
