# sc/treaty — Composite des traités (CA1 + CA6)

Fait : `Proposal` supprimé ; `Treaty { articles }` ; chaque `Article` sait `check`/`value`/`apply`/`label`
(sim-campaign/src/negotiation/*) ; poids dans `data/ai/diplomacy.json` (`treaty_weights`) ; ADR 0202.
Tests sim-campaign + ai verts (graines cv3_ai_stances 2,4,5 ; moitié « F4 » de g5_neighbors retirée).
Reste : fusion par l'orchestrateur.
