# C7c — documentation des colonies (manuel, codex)

Spec : `docs/design/2026-09-24-echelle-colonies.md`. ADR 0005. Suit C4 (`docs/wip/c4-core-settlements.md`),
C5 (`docs/wip/c5-settlements-ui.md`), C7a (`docs/wip/c7a-settlements-balance.md`).
Périmètre : `docs/manuel.md`, `data/codex/`, `game/scripts/ui/encyclopedia.gd` (fiches
« Mécaniques », hors `game/scripts/map` qui appartient à C7b). Toute valeur citée vérifiée dans
`data/settlements/rules.json` ou le code Rust (`core/crates/sim-campaign/src/settlements.rs`,
`movement.rs`, `economy.rs`, `orders.rs`, `siege.rs`).

## État

- [x] Lecture de la spec, de l'ADR, des wip C4/C5/C7a, de `rules.json` et du code Rust cité.
- [x] `docs/manuel.md` : nouvelle section 5 « Provinces et colonies » remplaçant l'ancienne
  section province/ville, avec les 5 types, le contrôle dérivé de la cité, le revenu par poids et
  le bonus de province complète, garnisons/entretien/plafond/ordre garnison, bâtiments permis,
  déplacement sur le graphe (routes, mer, portée d'une saison, hiver), repli après défaite, sièges
  des places secondaires, interface (panneau, onglet Colonies, clic droit, anneaux, paliers de
  zoom). Numérotation des sections suivantes décalée (+1).
- [x] Passages corrigés ailleurs dans le manuel (grep `province`/`garnison`) : § armées/ravitaillement,
  § batailles/sièges, § raccourcis.
- [x] Codex : `cdx_places_fortes` (hiérarchie réelle cité/ville close/bastide/château/abbaye/village,
  grounding historique des 5 types de colonie) et `cdx_deroute_debandade` (déroute/débandade
  historique, grounding de la règle de repli C7a), liés à `cdx_chevauchee`, `cdx_seigneurie`,
  `cdx_crecy`, `cdx_poitiers`, `cdx_du_guesclin`.
- [x] `game/scripts/ui/encyclopedia.gd` : fiches « Mécaniques » `mech_sieges` et `mech_economy`
  mises à jour (sièges et revenu par colonie, plus par province). Pas de compteur de mécaniques
  dans `game/tests/smoke.gd` à mettre à jour (vérifié : aucune assertion sur `MECHANICS` ou
  `mech_`).
- [x] Vérifications : `uv run --project tools pytest` (251 passed, 1 failed = l'échec connu et hors
  périmètre `test_portraits.py::test_dry_run_makes_no_network_call` ; `test_codex.py` valide les
  deux nouvelles fiches) ; `cd core && cargo test` (tout vert, aucun avertissement) ;
  `core/build.sh` ; `godot --headless --path game --import` ; `godot --headless --path game
  --script res://tests/smoke.gd` (exit 0, « codex, 233 entries » avec les deux nouvelles fiches,
  « mechanics 11 » inchangé).

## Points ouverts / écarts constatés

- Le compteur de mécaniques (`MECHANICS` dans `encyclopedia.gd`, `mechanics 11` dans le smoke) n'a
  pas bougé : je n'ai édité que le texte de `mech_economy` et `mech_sieges` (province -> colonie),
  sans ajouter d'entrée « Colonies » séparée, puisque le sujet est déjà couvert par les fiches
  Économie et Sièges existantes et que le manuel porte la documentation complète.
- Le manuel décrivait encore un modèle « une ville par province » dans toute la section 5 ; réécrite
  intégralement. D'autres passages corrigés : clic droit/anneaux (§4), recrutement par colonie (§6),
  armées alliées sur la même colonie (§6), sièges par colonie et village pris sans siège (§8).
- Deux entrées de Codex ajoutées, historiques et sourcées (pas de règle de jeu dans le corps, la
  simplification est notée dans `anachronism`) : `cdx_places_fortes` (hiérarchie réelle
  cité/ville close/bastide/château/abbaye/village, grounding des 5 types de colonie) et
  `cdx_deroute_debandade` (déroute et débandade après Crécy/Poitiers, grounding de la règle de
  repli C7a).
- Aucun écart trouvé entre la spec et le code pour les points documentés (repli, plafond de
  garnison, entretien par type, revenu par poids, bonus de province complète, prise immédiate d'un
  village, sièges des places secondaires) : tout est vérifié dans `rules.json` et
  `settlements.rs`/`movement.rs`/`economy.rs`/`orders.rs`/`siege.rs`.

## Prochaine étape

Terminé : fusion par l'orchestrateur (après C7b, qui touche `game/scripts/map`).
