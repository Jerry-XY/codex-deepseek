//! Gates access to the retained startup model catalog on current managed provider requirements.

use std::sync::Arc;

use codex_core::config::Config;
use codex_core::config::ConfigOverrides;
use codex_login::AuthManager;
use codex_models_manager::manager::RefreshStrategy;
use codex_models_manager::manager::SharedModelsManager;
use codex_protocol::openai_models::ModelPreset;

use crate::config_manager::ConfigManager;

/// Checks the retained catalog route before model/list and background refreshes.
pub(crate) struct ModelCatalog {
    config_manager: ConfigManager,
    config: Arc<Config>,
    models_manager: SharedModelsManager,
    first_party_auth: Option<Arc<AuthManager>>,
    first_party_catalog: tokio::sync::OnceCell<(Config, SharedModelsManager)>,
}

impl ModelCatalog {
    pub(crate) fn new(
        config_manager: ConfigManager,
        config: Arc<Config>,
        models_manager: SharedModelsManager,
    ) -> Self {
        Self {
            config_manager,
            config,
            models_manager,
            first_party_auth: None,
            first_party_catalog: tokio::sync::OnceCell::new(),
        }
    }

    pub(crate) fn with_first_party_auth(mut self, auth_manager: Arc<AuthManager>) -> Self {
        self.first_party_auth = Some(auth_manager);
        self
    }

    pub(crate) async fn list_models(
        &self,
        refresh_strategy: RefreshStrategy,
    ) -> std::io::Result<Vec<ModelPreset>> {
        // Check before consulting even a warm cache, just as turn admission checks
        // the retained session route before using it.
        self.config_manager
            .check_thread_model_provider(&self.config)
            .await?;
        // A managed external default chooses the inference route, not the whole picker.
        // Keep the first-party catalog on its own credentials and policy, then let
        // supported_models append public external aliases from the registry.
        if let Some(auth_manager) = &self.first_party_auth
            && !self.config.model_provider.is_openai()
            && !self.config.configured_external_models().is_empty()
        {
            let (config, manager) = self
                .first_party_catalog
                .get_or_try_init(|| async {
                    let mut config = self
                        .config_manager
                        .load_with_overrides(
                            /*request_overrides*/ None,
                            ConfigOverrides {
                                // Catalog-only config: an empty model prevents the user's external
                                // default alias from reapplying its route. No inference uses it.
                                model: Some(String::new()),
                                model_provider: Some("openai".into()),
                                ..Default::default()
                            },
                        )
                        .await?;
                    let external = self.config.configured_external_models();
                    if config.model_catalog.as_ref().is_some_and(|catalog| {
                        !catalog.models.is_empty()
                            && catalog.models.iter().all(|model| {
                                external.iter().any(|route| route.api_model == model.slug)
                            })
                    }) {
                        config.model_catalog = None;
                    }
                    let manager =
                        codex_core::build_models_manager(&config, Arc::clone(auth_manager));
                    Ok::<_, std::io::Error>((config, manager))
                })
                .await?;
            self.config_manager
                .check_thread_model_provider(config)
                .await?;
            return Ok(manager
                .list_models(refresh_strategy, config.http_client_factory())
                .await);
        }
        Ok(self
            .models_manager
            .list_models(refresh_strategy, self.config.http_client_factory())
            .await)
    }
}
