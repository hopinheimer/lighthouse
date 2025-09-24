use std::sync::Arc;
use tokio::runtime::{Builder as RuntimeBuilder, Runtime};

pub struct LeanEnvironmentBuilder {
    runtime: Option<Arc<Runtime>>,
}

pub struct LeanEnvironment {
    pub runtime: Arc<Runtime>,
}

impl LeanEnvironmentBuilder {
    pub fn lean() -> Self {
        Self { runtime: None }
    }

    pub fn runtime(mut self) -> Result<Self, String> {
        self.runtime = Some(Arc::new(
            RuntimeBuilder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("Failed to start runtime: {:?}", e))?,
        ));
        Ok(self)
    }

    pub fn build(self) -> Result<LeanEnvironment, String> {
        Ok(LeanEnvironment {
            runtime: self
                .runtime
                .ok_or("cannot build environment without runtime")?,
        })
    }
}
