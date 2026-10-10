use super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn apply_preserves_explicit_tool_capabilities_after_registry_round_trip() {
    let home = tempfile::tempdir().unwrap();
    let model = model_from_args(
        "gateway-flash".into(),
        Some("vendor-flash".into()),
        ModelOptions {
            tool_mode: Some("code_mode".into()),
            supports_search_tool: Some(true),
            ..Default::default()
        },
        /*existing*/ None,
    )
    .unwrap();
    let registry = ProviderRegistry {
        schema_version: REGISTRY_VERSION,
        providers: BTreeMap::from([(
            "gateway".into(),
            ManagedProvider {
                name: "Gateway".into(),
                base_url: "https://gateway.example.com/v1".into(),
                env_key: "GATEWAY_API_KEY".into(),
                supports_websockets: false,
                models: vec![model],
            },
        )]),
    };
    save_registry(home.path(), &registry).unwrap();
    let loaded = load_registry(home.path()).unwrap();
    apply_registry(home.path(), &loaded).await.unwrap();
    let catalog: ModelsResponse = serde_json::from_str(
        &std::fs::read_to_string(home.path().join(CATALOG_DIR).join("gateway.json")).unwrap(),
    )
    .unwrap();
    let info = &catalog.models[0];
    assert_eq!(
        (info.tool_mode, info.supports_search_tool),
        (Some(ToolMode::CodeMode), true)
    );

    let edited = model_from_args(
        "gateway-flash".into(),
        /*api_model*/ None,
        ModelOptions {
            display_name: Some("Renamed Flash".into()),
            ..Default::default()
        },
        Some(loaded.providers["gateway"].models[0].clone()),
    )
    .unwrap();
    let renamed = build_model_info(&edited).unwrap();
    assert_eq!(
        (renamed.tool_mode, renamed.supports_search_tool),
        (info.tool_mode, info.supports_search_tool)
    );
}

#[test]
fn explicit_direct_mode_and_search_false_override_template() {
    let mut model = deepseek_provider().unwrap().models.remove(0);
    model.tool_mode = Some(ToolMode::Direct);
    model.supports_search_tool = Some(false);
    let info = build_model_info(&model).unwrap();
    assert_eq!(
        (info.tool_mode, info.supports_search_tool),
        (Some(ToolMode::Direct), false)
    );
}

#[test]
fn legacy_registry_uses_template_tool_capabilities() {
    let model = deepseek_provider().unwrap().models.remove(0);
    let catalog: ModelsResponse = serde_json::from_str(DEEPSEEK_CATALOG).unwrap();
    let template = catalog
        .models
        .iter()
        .find(|info| Some(&info.slug) == model.template.as_ref())
        .unwrap();
    let actual = build_model_info(&model).unwrap();
    assert_eq!(
        (actual.tool_mode, actual.supports_search_tool),
        (template.tool_mode, template.supports_search_tool)
    );
}
