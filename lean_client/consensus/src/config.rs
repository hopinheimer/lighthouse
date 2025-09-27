#[derive(Clone, Debug)]
pub struct LeanConfig {
    pub num_validators: u64,
    pub genesis_time: u64,
    // Time parameters from leanSpec
    pub slot_duration_ms: u64,
    pub seconds_per_slot: u64,
    pub justification_lookback_slots: u64,
    // Timing cutoffs (in basis points)
    pub proposer_reorg_cutoff_bps: u64,
    pub vote_due_bps: u64,
    pub fast_confirm_due_bps: u64,
    pub view_freeze_cutoff_bps: u64,
    // State list length presets
    pub historical_roots_limit: u64,
    pub validator_registry_limit: u64,
}

impl LeanConfig {
    /// Load the default DEVNET configuration from leanSpec
    pub fn devnet() -> Self {
        // Constants from leanSpec
        const SLOT_DURATION_MS: u64 = 4000;
        const SECONDS_PER_SLOT: u64 = SLOT_DURATION_MS / 1000;
        const JUSTIFICATION_LOOKBACK_SLOTS: u64 = 3;
        const PROPOSER_REORG_CUTOFF_BPS: u64 = 2500;
        const VOTE_DUE_BPS: u64 = 5000;
        const FAST_CONFIRM_DUE_BPS: u64 = 7500;
        const VIEW_FREEZE_CUTOFF_BPS: u64 = 7500;
        const HISTORICAL_ROOTS_LIMIT: u64 = 1 << 18; // 2^18
        const VALIDATOR_REGISTRY_LIMIT: u64 = 1 << 12; // 2^12

        Self {
            // Default values for existing fields
            num_validators: 64,
            genesis_time: 0,
            // leanSpec configuration
            slot_duration_ms: SLOT_DURATION_MS,
            seconds_per_slot: SECONDS_PER_SLOT,
            justification_lookback_slots: JUSTIFICATION_LOOKBACK_SLOTS,
            proposer_reorg_cutoff_bps: PROPOSER_REORG_CUTOFF_BPS,
            vote_due_bps: VOTE_DUE_BPS,
            fast_confirm_due_bps: FAST_CONFIRM_DUE_BPS,
            view_freeze_cutoff_bps: VIEW_FREEZE_CUTOFF_BPS,
            historical_roots_limit: HISTORICAL_ROOTS_LIMIT,
            validator_registry_limit: VALIDATOR_REGISTRY_LIMIT,
        }
    }
}
