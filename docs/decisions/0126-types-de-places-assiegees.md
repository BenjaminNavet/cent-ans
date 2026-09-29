# ADR 0126 — Types de places assiégées (château, bourg fortifié, cité)

Date : 2026-09-29. Statut : accepté. Lot NT1 du chantier NT
(`docs/superpowers/specs/2026-09-29-nt-nuit-tww3-design.md`). Suite des ADR 0026 (villes
emblématiques) et 0047 (ville de siège dense).

## Contexte

Toutes les places non emblématiques partageaient le même plan : octogone, rues rayonnantes,
anneaux d'îlots (`SiegeWorks::generate`, BR3). Un château de Gascogne et une grande ville
d'Italie se ressemblaient ; seul Paris, Londres et les cinq autres plans historiques variaient.

## Décision

- **Trois types** (`PlaceKind`, `core/crates/sim-battle/src/siege_layouts.rs`) :
  - **cité** : le plan en anneaux actuel, inchangé (y compris son tirage de rayon par le flux de
    la bataille) ;
  - **bourg fortifié** : enceinte polygonale (6 à 9 côtés), grand-rue de la porte assaillie à une
    porte fortifiée (tour de porte) du côté opposé, coudée à mi-chemin, par la place du marché ;
    deux ruelles parallèles et parfois une rue transversale ; maisons en bandes le long des rues
    puis contre le rempart, jardins libres derrière ; église paroissiale ;
  - **château** : enceinte serrée (5 à 7 côtés, rayon 80-94 m), courtines plus hautes, basse-cour
    au centre (place de capture), donjon carré vers le fond, quelques bâtiments adossés à la
    courtine, pas d'église.
- **Choix par les données** : bloc `places` de `data/rules/siege_town.json` (schéma
  `siege_town_rules.schema.json`) : type selon le genre de la localité (`city` → cité, `town` et
  `abbey` et `village` → bourg, `castle` → château), une ville faible (fortification ≤ 1) tracée
  en bourg, un bourg fort (≥ 4) en cité. La campagne le calcule dans `battle_request.rs` et le
  transmet par `SiegeSetup.place` (absent = cité : anciennes parties, rejeux, tests).
- **Plans emblématiques prioritaires** : `SiegeWorks::for_battle` garde le plan historique
  quand il existe ; le type ne sert qu'aux autres places.
- **Variations déterministes par province** : `place_seed` (FNV-1a de l'identifiant de province)
  alimente un hachage (`town::hash01`) pour le nombre de côtés, le rayon, la rotation, la position
  de la porte, le coude de la grand-rue, l'écart des ruelles, la position et la taille du donjon
  et le choix des bâtiments. Le flux aléatoire de la bataille n'est pas touché (seule la brèche de
  campagne l'utilise, comme avant) : une province garde toujours le même plan. Le côté de la porte
  assaillie fait toujours face à l'assaillant (−z), ce que supposent le déploiement et l'IA.
- **Donjon** : une `House` marquée `keep` (emprise carrée, obstacle du cheminement et des
  figurines) qui ne brûle jamais ; le rendu la dessine en tour agrandie (même maquette que les
  tours de l'enceinte). Les rues du bourg et du château sont exposées (`SiegeWorks.streets`) pour
  que le marché laisse leurs débouchés libres.
- **Module** : `siege_layouts.rs` (générateurs), à côté de `siege_layout.rs` (plans
  emblématiques, ADR 0026) ; `siege.rs` n'a reçu que les champs nouveaux.

## Conséquences

- `SiegeSetup`, `SiegeWorks` et `House` gagnent des champs à défaut sérialisé (`place`,
  `streets`, `keep`) : sauvegardes et rejeux antérieurs restent lisibles et donnent une cité.
- Le pont expose `get_siege().place` et `houses[].keep` ; `debug_stage_place_siege` monte un siège
  de chaque type pour les captures (`game/tests/nt1_siege_shot.gd`).
- Le bourg est volontairement moins dense que la cité (≈ 40 îlots contre ≈ 150) ; la densité se
  règle dans `places.borough`. Équilibre des assauts contre un château (petite enceinte, garnison
  serrée) à surveiller en partie pilote.
