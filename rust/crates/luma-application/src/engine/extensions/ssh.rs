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
            action_payload: Some(serde_json::json!({ "alias": alias })),
        };
        let action = luma_domain::ActionDescriptor {
            id: luma_domain::ActionId::new("record_connection"),
            label: "Record".into(),
            risk: luma_domain::ActionRisk::Safe,
            confirmation: false,
        };
        let cancel = self.inner.lock().await.session_cancel.child_token();
        let _ = module
            .perform(
                crate::module::ActionRequest {
                    result,
                    action,
                    confirmation: false,
                },
                cancel,
            )
            .await;
    }
}
