use super::{
    confirm_action, identity_payload, safe_action, DatabasePortalTarget, DatabasePortalsModule,
    MODULE_ID,
};
use luma_domain::{ActionRisk, ModuleId, SearchItem};

impl DatabasePortalsModule {
    pub(super) async fn rehydrate_recall_item(
        &self,
        object_id: &str,
    ) -> Result<Option<SearchItem>, String> {
        let Some(id) = object_id
            .strip_prefix("db:")
            .and_then(|value| value.parse::<i64>().ok())
        else {
            return Ok(None);
        };
        let portal = self.repository.get(id).map_err(|error| error.to_string())?;
        Ok(portal.map(|portal| {
            let (kind, target) = match &portal.target {
                DatabasePortalTarget::Sqlite { path } => ("sqlite", path.display().to_string()),
                DatabasePortalTarget::Postgres {
                    host,
                    port,
                    database,
                    username,
                    ..
                } => (
                    "postgres",
                    format!("{username}@{host}:{port}/{database} · libpq auth"),
                ),
            };
            let production = portal.environment == "production";
            SearchItem {
                id: luma_domain::ResultId::new(object_id),
                module_id: ModuleId::new(MODULE_ID),
                title: portal.label.clone(),
                subtitle: Some(format!("{kind} · {} · {target}", portal.environment)),
                kind: "database_portal".into(),
                score: 0.0,
                primary_action: if production {
                    confirm_action("open_cli", "Open CLI", ActionRisk::Confirm)
                } else {
                    safe_action("open_cli", "Open CLI")
                },
                secondary_actions: vec![],
                ui_intent: None,
                action_payload: Some(identity_payload(&portal)),
            }
        }))
    }
}
