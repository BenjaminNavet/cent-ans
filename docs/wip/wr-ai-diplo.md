# WR ai-diplo — état

FAIT sur `wr/ai-diplo` (ADR 0302) : `diplomacy/ai_pacts.rs` (JoinWar + non-agression IA), `ai_pacts` dans `data/ai/diplomacy.json`,
impôt provincial IA à 70 (cause : règles IA compilées, `AiCampaign::bundled()`), mesures de pactes/impôts dans `campaign_probe`.
Tests : `sim-campaign/tests/diplomacy/wr_ai_diplo.rs` (7), `ai/tests/economy/wr_province_tax_ai.rs`.
Reste : assouplir JoinWar seulement avec mesure de la sonde (risque de guerre de blocs) ; fusion par l'orchestrateur.
