use crate::module::LumaModule;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    DuplicateModuleId(String),
    DuplicateTrigger {
        trigger: String,
        existing_module: String,
        new_module: String,
    },
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::DuplicateModuleId(id) => write!(f, "duplicate module id: {id}"),
            RegistryError::DuplicateTrigger {
                trigger,
                existing_module,
                new_module,
            } => write!(
                f,
                "duplicate trigger `{trigger}`: already owned by {existing_module}, refused for {new_module}"
            ),
        }
    }
}

impl std::error::Error for RegistryError {}

#[derive(Default)]
pub struct ModuleRegistry {
    modules: HashMap<String, Arc<dyn LumaModule>>,
    enabled: HashMap<String, bool>,
    /// Lowercased trigger → module id for collision checks and routing.
    triggers: HashMap<String, String>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, module: Arc<dyn LumaModule>) -> Result<(), RegistryError> {
        let id = module.manifest().id.as_str().to_string();
        if self.modules.contains_key(&id) {
            return Err(RegistryError::DuplicateModuleId(id));
        }
        for trigger in &module.manifest().triggers {
            let key = trigger.to_lowercase();
            if let Some(existing) = self.triggers.get(&key) {
                return Err(RegistryError::DuplicateTrigger {
                    trigger: key,
                    existing_module: existing.clone(),
                    new_module: id,
                });
            }
        }
        for trigger in &module.manifest().triggers {
            self.triggers.insert(trigger.to_lowercase(), id.clone());
        }
        let enabled = module.manifest().default_enabled;
        self.enabled.insert(id.clone(), enabled);
        self.modules.insert(id, module);
        Ok(())
    }

    pub fn set_enabled(&mut self, module_id: &str, enabled: bool) -> bool {
        if self.modules.contains_key(module_id) {
            self.enabled.insert(module_id.to_string(), enabled);
            true
        } else {
            false
        }
    }

    pub fn is_enabled(&self, module_id: &str) -> bool {
        self.enabled.get(module_id).copied().unwrap_or(false)
    }

    pub fn get(&self, module_id: &str) -> Option<Arc<dyn LumaModule>> {
        self.modules.get(module_id).cloned()
    }

    pub fn list(&self) -> Vec<(String, bool, String)> {
        let mut rows: Vec<_> = self
            .modules
            .values()
            .map(|m| {
                let id = m.manifest().id.as_str().to_string();
                let enabled = self.is_enabled(&id);
                let name = m.manifest().display_name.clone();
                (id, enabled, name)
            })
            .collect();
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        rows
    }

    /// Disable enabled modules that declare missing runtime capabilities.
    pub fn apply_capability_preflight(
        &mut self,
        caps: &dyn crate::ports::CapabilityPort,
    ) -> Vec<(String, String)> {
        let ids: Vec<String> = self.modules.keys().cloned().collect();
        let mut denied = Vec::new();
        for id in ids {
            if !self.is_enabled(&id) {
                continue;
            }
            let Some(module) = self.modules.get(&id) else {
                continue;
            };
            for cap in &module.manifest().required_capabilities {
                if !caps.has(cap) {
                    self.enabled.insert(id.clone(), false);
                    denied.push((id.clone(), format!("missing capability: {cap}")));
                    break;
                }
            }
        }
        denied
    }

    pub fn list_module_info(&self) -> Vec<luma_protocol::ModuleInfoDto> {
        let mut rows: Vec<_> = self
            .modules
            .values()
            .map(|m| {
                let man = m.manifest();
                let id = man.id.as_str().to_string();
                luma_protocol::ModuleInfoDto {
                    id: id.clone(),
                    display_name: man.display_name.clone(),
                    enabled: self.is_enabled(&id),
                    glyph: man.workbench.glyph.clone(),
                    suggested_query: man.workbench.suggested_query.clone(),
                    empty_hint: man.workbench.empty_hint.clone(),
                    supports_browse: man.workbench.supports_browse,
                    triggers: man.triggers.clone(),
                }
            })
            .collect();
        rows.sort_by(|a, b| a.id.cmp(&b.id));
        rows
    }

    pub fn resolve_trigger(&self, token: &str) -> Option<Arc<dyn LumaModule>> {
        let token = token.to_lowercase();
        let module_id = self.triggers.get(&token)?;
        if !self.is_enabled(module_id) {
            return None;
        }
        self.modules.get(module_id).cloned()
    }

    /// All registered triggers (enabled or not) for query routing.
    pub fn all_triggers(&self) -> Vec<String> {
        let mut keys: Vec<_> = self.triggers.keys().cloned().collect();
        keys.sort();
        keys
    }

    pub fn contributing(&self) -> Vec<Arc<dyn LumaModule>> {
        use crate::module::SearchMode;
        self.modules
            .values()
            .filter(|m| {
                let id = m.manifest().id.as_str();
                self.is_enabled(id)
                    && matches!(m.manifest().search_mode, SearchMode::GlobalContributing)
            })
            .cloned()
            .collect()
    }

    pub fn all_modules(&self) -> Vec<Arc<dyn LumaModule>> {
        self.modules.values().cloned().collect()
    }

    /// Enabled modules only (for session warmup / active work).
    pub fn enabled_modules(&self) -> Vec<Arc<dyn LumaModule>> {
        self.modules
            .values()
            .filter(|m| self.is_enabled(m.manifest().id.as_str()))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::{
        ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, SearchMode,
        SearchSink, WarmupContext,
    };
    use async_trait::async_trait;
    use luma_domain::{ActionDescriptor, ModuleId, Query, SearchItem};
    use tokio_util::sync::CancellationToken;

    struct StubModule {
        manifest: ModuleManifest,
    }

    #[async_trait]
    impl LumaModule for StubModule {
        fn manifest(&self) -> &ModuleManifest {
            &self.manifest
        }
        async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
            ModuleState::Ready
        }
        async fn search(&self, _query: Query, _sink: SearchSink, _cancel: CancellationToken) {}
        async fn actions(&self, _result: &SearchItem) -> Vec<ActionDescriptor> {
            vec![]
        }
        async fn perform(
            &self,
            _action: ActionRequest,
            _cancel: CancellationToken,
        ) -> ActionOutcome {
            ActionOutcome::Cancelled
        }
        async fn teardown(&self) {}
    }

    fn stub(id: &str, triggers: &[&str]) -> Arc<dyn LumaModule> {
        Arc::new(StubModule {
            manifest: ModuleManifest {
                id: ModuleId::new(id),
                display_name: id.into(),
                triggers: triggers.iter().map(|t| (*t).to_string()).collect(),
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: Default::default(),
            },
        })
    }

    #[test]
    fn rejects_duplicate_module_id() {
        let mut reg = ModuleRegistry::new();
        reg.register(stub("luma.a", &["a"])).unwrap();
        let err = reg.register(stub("luma.a", &["b"])).unwrap_err();
        assert!(matches!(err, RegistryError::DuplicateModuleId(_)));
    }

    #[test]
    fn rejects_duplicate_trigger() {
        let mut reg = ModuleRegistry::new();
        reg.register(stub("luma.a", &["x"])).unwrap();
        let err = reg.register(stub("luma.b", &["X"])).unwrap_err();
        assert!(matches!(err, RegistryError::DuplicateTrigger { .. }));
    }

    #[test]
    fn capability_preflight_disables_module_missing_cap() {
        use crate::ports::FakeCapabilities;
        let mut reg = ModuleRegistry::new();
        reg.register(Arc::new(StubModule {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.ax"),
                display_name: "AX".into(),
                triggers: vec!["ax".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec!["accessibility".into()],
                workbench: Default::default(),
            },
        }))
        .unwrap();
        let denied = reg.apply_capability_preflight(&FakeCapabilities {
            accessibility: false,
            keychain: true,
        });
        assert_eq!(denied.len(), 1);
        assert!(!reg.is_enabled("luma.ax"));
    }
}
