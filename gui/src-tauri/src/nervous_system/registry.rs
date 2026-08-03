use std::sync::Arc;
use crate::nervous_system::provider::SandboxProvider;

pub struct ProviderRegistry {
    providers: Vec<Arc<dyn SandboxProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        let mut registry = Self { providers: Vec::new() };
        registry.register_defaults();
        registry
    }

    pub fn register(&mut self, provider: Arc<dyn SandboxProvider>) {
        self.providers.push(provider);
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn SandboxProvider>> {
        self.providers.iter().find(|p| p.id() == id).map(|p| p.clone())
    }

    pub fn all(&self) -> &[Arc<dyn SandboxProvider>] {
        &self.providers
    }

    fn register_defaults(&mut self) {
        self.providers.push(Arc::new(super::providers::process::ProcessJailProvider));
        self.providers.push(Arc::new(super::providers::docker_podman_qemu::DockerProvider::new()));
        self.providers.push(Arc::new(super::providers::docker_podman_qemu::PodmanProvider::new()));
        self.providers.push(Arc::new(super::providers::qemu::QemuProvider::new()));
    }
}
