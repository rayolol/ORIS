use crate::hal::KernelError;
use crate::config::ActuatorConfig;

#[allow(async_fn_in_trait)]
pub trait BackendContainer {
    async fn init(&mut self, config: &ActuatorConfig) -> Result<(), KernelError>;
}
