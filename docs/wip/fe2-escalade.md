# FE2 — escalade de guerre et guerre privée

Branche `feat/fe2-escalade` (depuis `main` dfe88244). Spec § 4.3, plan F2.

## État
- Règles : `feudal.json` / schéma / `FeudalRules::escalation` (`EscalationRules`, `ProtectionScore`,
  `ArbitrationRules`).
- `feudal.rs` (ajouts en fin de fichier) : `adjust_loyalty`, `common_liege`, `protection_score`
  (score IA provisoire, remplacé en F5), `likelihood_of`, `ai_arbitration`, `war_escalation_preview`,
  `escalate_war`, `call_liege` (cascade), `intervene`, `shirk`, `summon_host`, `apply_arbitration`,
  `refuse_feudal_call`, `CampaignState::arbitrate`.
- `diplomacy.rs` : `Proposal::Protection` / `Proposal::Arbitration` (offres au joueur, hors délai de
  grâce) ; `declare_war` appelle `feudal::escalate_war` ; `call_to_arms` saute le suzerain féodal
  (déjà appelé). Refus ou expiration : dérobade / laisser faire.
- `orders.rs` : `Order::ArbitratePrivateWar { offer, verdict }`.
- Tests `feudal_escalation.rs` écrits (4).

## Prochaine étape
- Compiler, faire passer les tests, fmt/clippy/test workspace, commit final `FE2: …`.

## Points ouverts
- `summon_host` (titres) double `rally_vassals` (champ `suzerain`) : à unifier à la fusion avec F1.
- Le vassal abandonné qui change d'allégeance (§ 4.3.2) : F3/F5.
- Pont Godot (offres, `ArbitratePrivateWar`, aperçu) : F6.
