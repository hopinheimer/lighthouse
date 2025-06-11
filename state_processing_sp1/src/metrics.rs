// SP1-compatible no-op metrics module
// All metrics operations are stubbed out for zkVM compatibility

use std::sync::LazyLock;

// No-op metric types for SP1 compatibility
pub struct IntGauge;
pub struct Histogram;

impl IntGauge {
    pub fn set(&self, _val: i64) {}
    pub fn inc(&self) {}
    pub fn dec(&self) {}
}

impl Histogram {
    pub fn start_timer(&self) -> HistogramTimer {
        HistogramTimer
    }
    pub fn observe(&self, _val: f64) {}
}

pub struct HistogramTimer;

impl HistogramTimer {
    pub fn observe_duration(self) {}
}

// No-op constructor functions
pub fn try_create_int_gauge(_name: &str, _help: &str) -> Result<IntGauge, ()> {
    Ok(IntGauge)
}

pub fn try_create_histogram(_name: &str, _help: &str) -> Result<Histogram, ()> {
    Ok(Histogram)
}

/*
 * Participation Metrics (no-op)
 */
pub static PARTICIPATION_PREV_EPOCH_HEAD_ATTESTING_GWEI_TOTAL: LazyLock<Result<IntGauge, ()>> =
    LazyLock::new(|| Ok(IntGauge));

pub static PARTICIPATION_PREV_EPOCH_TARGET_ATTESTING_GWEI_TOTAL: LazyLock<Result<IntGauge, ()>> =
    LazyLock::new(|| Ok(IntGauge));

pub static PARTICIPATION_PREV_EPOCH_SOURCE_ATTESTING_GWEI_TOTAL: LazyLock<Result<IntGauge, ()>> =
    LazyLock::new(|| Ok(IntGauge));

pub static PARTICIPATION_CURRENT_EPOCH_TOTAL_ACTIVE_GWEI_TOTAL: LazyLock<Result<IntGauge, ()>> =
    LazyLock::new(|| Ok(IntGauge));

/*
 * Processing metrics (no-op)
 */
pub static PROCESS_EPOCH_TIME: LazyLock<Result<Histogram, ()>> = 
    LazyLock::new(|| Ok(Histogram));

pub static BUILD_EPOCH_CACHE_TIME: LazyLock<Result<Histogram, ()>> = 
    LazyLock::new(|| Ok(Histogram));

pub static BUILD_PROGRESSIVE_BALANCES_CACHE_TIME: LazyLock<Result<Histogram, ()>> =
    LazyLock::new(|| Ok(Histogram));

/*
 * Participation Metrics (progressive balances) (no-op)
 */
pub static PARTICIPATION_PREV_EPOCH_TARGET_ATTESTING_GWEI_PROGRESSIVE_TOTAL: LazyLock<Result<IntGauge, ()>> = 
    LazyLock::new(|| Ok(IntGauge));

pub static PARTICIPATION_CURR_EPOCH_TARGET_ATTESTING_GWEI_PROGRESSIVE_TOTAL: LazyLock<Result<IntGauge, ()>> = 
    LazyLock::new(|| Ok(IntGauge));
