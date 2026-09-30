# Sonde de qualité de l'IA de campagne (`ia_quality_probe`)

Branche `feat/ia`. Exemple `core/crates/ai/examples/ia_quality_probe.rs` : mesure la qualité du
jeu de l'IA (prises manquées, pertes sans secours, armées oisives, batailles suicidaires,
fragmentation, sièges, trésor, coût par tour), IA contre IA depuis 1337.

## État
- [x] Squelette
- [ ] Métriques
- [ ] Mesures graines 1-2, 60 tours

## Prochaine étape
Implémenter les métriques (enregistreur appelé dans le planificateur de `end_turn_with`).
