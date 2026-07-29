use super::super::*;

impl Engine {
    pub(crate) async fn handle_ssh_session_ended(&self, alias: String, exit_code: i32) {
        if exit_code != 0 {
            return;
        }
        let module = {
            let g = self.inner.lock().await;
            if !g.registry.is_enabled("luma.ssh") {
                return;
            }
            g.registry.get("luma.ssh")
        };
        let Some(module) = module else {
            return;
        };
        let result = luma_domain::SearchItem {
            id: luma_domain::ResultId::new(format!("ssh:record:{alias}")),
            module_id: luma_domain::ModuleId::new("luma.ssh"),
            title: alias.clone(),
            subtitle: None,
            kind: "internal".into(),
            score: 0.0,
            primary_action: luma_domain::ActionDescriptor {
                id: luma_domain::ActionId::new("record_connection"),
                label: "Record".into(),
                risk: luma_domain::ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: Some(serde_json::json!({ "alias": alias.clone() })),
        };
        let action = luma_domain::ActionDescriptor {
            id: luma_domain::ActionId::new("record_connection"),
            label: "Record".into(),
            risk: luma_domain::ActionRisk::Safe,
            confirmation: false,
        };
        let cancel = self.inner.lock().await.session_cancel.child_token();
        let outcome = module
            .perform(
                crate::module::ActionRequest {
                    result,
                    action,
                    confirmation: false,
                },
                cancel,
            )
            .await;
        if !matches!(outcome, crate::module::ActionOutcome::Success { .. }) {
            return;
        }
        let Some(repo) = self.recall.as_ref() else {
            return;
        };
        let object_id = format!("ssh:{alias}");
        if let Ok(Some(item)) = module.rehydrate_recall(&object_id).await {
            let now_unix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_secs() as i64)
                .unwrap_or(0);
            if let Some(object) = super::super::recall::recall_object_from_item(&item, now_unix) {
                let _ = repo.record_success(object);
            }
        }
    }
}
