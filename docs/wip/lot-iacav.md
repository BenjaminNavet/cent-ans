# Lot IA-CAV (boucle juge, iter 2) : charge groupée de la cavalerie

## État : TERMINÉ (à fusionner)
- Code : `core/crates/sim-battle/src/ai/mass.rs` (alignement hors de portée, départ ensemble, tenaille sur les
  flancs quand pieux, armées de cavalerie seule), trot vers le poste d'aile dans `ai/horse.rs`.
- Données : `data/rules/battle_mass_charge.json` + schéma. ADR : `docs/decisions/XXXX-iacav-charge-groupee-cavalerie.md`
  (à numéroter à la fusion).
- Mesures et tests : voir l'ADR. fmt, clippy `-D warnings`, `cargo test --workspace` verts ; seed bots rapide passé.
- Fixture `auto_resolve_scenarios.json` non régénérée : l'exemple `balance_probe` cité dans son commentaire n'existe
  plus ; le test de calibration (`tests/replay/auto_resolve_calibration.rs`) passe ; la fixture est déjà périmée
  (ADR 0792). À refaire dans un lot dédié.

## Ouvert
- Hommes d'armes à pied balayés par une charge frontale égale (8K/8A 8/8, 4-6 pertes) : modèle de choc (ADR 0711).
- Armées mixtes (avec infanterie) : l'ancienne charge un à un reste ; la charge groupée y fit tomber
  `symmetric_flat_battle_is_open` à 0/10 (à retravailler avec l'attente de l'infanterie).
