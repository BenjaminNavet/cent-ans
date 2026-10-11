# RG13c — salle du conseil (UI Godot)

Spec : ADR 0341 § 3-4 (lu dans le checkout principal, non commité dans cette branche).

## État
- `game/scripts/ui/council_room.gd` : salle 3D (pierre, colonnes, nervures, table de chêne, carte,
  chandeliers + OmniLights chauds, rai de jour froid, tapisseries, brume), figures GLB ou marionnettes,
  pointage par rayon, surbrillance (material_overlay), caméra fixe au pied de la table + balancement.
- `game/scripts/ui/council_chamber.gd` + `scenes/ui/council_chamber.tscn` : fenêtre (salle + fiche
  latérale, carte flottante au survol, conseils filtrés par `advice_domains`, Nommer / Révoquer).
- `game/scripts/map/council_controller.gd` : bouton « Conseil », touche **J** (K = Codex, déjà prise),
  alerte `council_vacancy` (HudController), conseils = alertes + indice courant.
- Tests : `tests/council_chamber_test.gd`, capture `tests/council_shot.gd`.

## Reste
- Brancher sur le vrai pont quand `get_council` existe (garde `has_method`, démo sinon).
- Le cœur doit produire l'alerte `kind = "council_vacancy"` ; les `advice_domains` doivent être des `kind` d'alertes.
- Regard visuel de la session principale (capture), modèles GLB `council_*.glb` (orientation supposée : face vers -Z).
