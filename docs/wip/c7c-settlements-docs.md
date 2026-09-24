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
- [ ] Vérifications : pytest, cargo test, build.sh, import Godot, smoke Godot.

## Points ouverts / écarts constatés

- (à compléter après vérification)

## Prochaine étape

Lancer les vérifications (§ 3 de la tâche), corriger si besoin, puis rapport final.
