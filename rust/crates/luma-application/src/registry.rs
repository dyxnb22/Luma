use crate::module::LumaModule;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Default)]
pub struct ModuleRegistry {
    modules: HashMap<String, Arc<dyn LumaModule>>,
    enabled: HashMap<String, bool>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, module: Arc<dyn LumaModule>) {
        let id = module.manifest().id.as_str().to_string();
        let enabled = module.manifest().default_enabled;
        self.enabled.insert(id.clone(), enabled);
        self.modules.insert(id, module);
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

    pub fn resolve_trigger(&self, token: &str) -> Option<Arc<dyn LumaModule>> {
        let token = token.to_lowercase();
        self.modules.values().find_map(|m| {
            let id = m.manifest().id.as_str();
            if !self.is_enabled(id) {
                return None;
            }
            if m.manifest()
                .triggers
                .iter()
                .any(|t| t.eq_ignore_ascii_case(&token))
            {
                Some(m.clone())
            } else {
                None
            }
        })
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
}
