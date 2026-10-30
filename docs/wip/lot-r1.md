# Lot R1 — renommée accumulée, actes à dates fixes, ambitions (audit 2026-10-29-campagne, R1-0/1/2)

Branche : `worktree-agent-adc1d3f985235a4e2` (worktree agent). Source : `docs/audits/2026-10-29-campagne.md` § 4 R1, § 6.

## État (30/10, après reprise et rebase sur main)
- [x] R1-0 sonde officielle (`seed-bots/src/timeline.rs`, `examples/campaign_timeline.rs`)
- [x] Mesures « avant » (out/r1/before, sonde + données de 95b504043) et « après » (out/r1/after3) : tableau dans l'ADR 0805
- [x] R1-1 renommée, actes à dates fixes, score d'acte signé dans le règne ; tests acts:: verts
- [x] R1-2 ambitions à effets ; test `seed-bots --test ambitions` vert (36/36 > 10 %)
- [x] Sauvegarde v12 + migration + fixture `save_v12.json`
- [x] ADR 0805 (mesures, seuil 43), INDEX et statuts de 0784/0785/0788/0793
- [x] Vérification complète : fmt, clippy, tests workspace (2 assertions `STATE_VERSION == 11` passées à 12), seed bots rapides 0 constat
- [ ] Passe longue seed bots en arrière-plan (lancée par l'agent, constats à présenter)

## Coordination avec le lot TUTO (prestige de faction)
La renommée de maison sera `CampaignState::house_renown(data)` (entier, jauge) ; son journal est
`state.acts.renown` (faits : tour, source, points, cause). Le lot TUTO peut afficher un champ `prestige` de faction
provisoire ; à terme il sera remplacé par cette renommée (pas de doublon de règle dans `core/`).

## Prochaine étape
Lire le rapport de la passe longue (target/seed-bots/) ; numéroter l'ADR 0805 à la fusion.

## Client (à afficher plus tard)
`ActView`/`TransitionView` gagnent `renown`, `house_renown`, `renown_recent` ; `ActPolicyRow` gagne
`renown_bonus_percent` ; deux ambitions à choisir (`policy_pending` tant qu'il en manque). Noms de champs inchangés.
