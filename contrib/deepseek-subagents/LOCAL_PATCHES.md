# Local patch stack for Codex 0.162.1

This branch reuses `codex-v0.162.0-deepseek.1` (commit
`316fda55747ceebb6bf11f12a2f41cd20ea074df`) and merges the official
`rust-v0.162.1` tag (`092d3acd6bec3e3a14bdc7e7a2810ab628ab759d`).
The provider and external-subagent implementation stays in the community
history. Local fixes are separate commits:

1. Preserve explicit `tool_mode` and `supports_search_tool` during provider edits
   and catalog regeneration.
2. Preserve an explicit `multi_agent_version`, including V1, during regeneration.
3. Preserve bounded endpoint guidance in canonical model instructions and the
   serialized legacy instruction field. The suffix limit is 1024 UTF-8 bytes.
4. Keep the first-party model picker available when a managed external provider
   is the default. Its catalog uses its own credentials and policy checks;
   inference keeps the configured external route.

## Configure an already registered gateway

The following is for a gateway already verified to accept Codex's `exec`
custom tool and deferred connector discovery. It is not a universal capability
preset for every DeepSeek endpoint. Replace `gateway` and the public model ID
with the entries in your provider registry.

```sh
codex provider model edit gateway DeepSeek-V4.1-Flash \
  --tool-mode code_mode \
  --supports-search-tool true \
  --multi-agent-version v1 \
  --supports-images true \
  --instructions-suffix 'Always call view_image directly when it is provided as a top-level tool; never wrap that image call in exec. Use exec and ALL_TOOLS to discover and call deferred tools.'
codex provider apply
codex provider validate
```

Omitted capability fields retain the template's behavior. Explicit `direct`
and `false` can override capabilities. Do not enable custom `exec` solely
because an endpoint supports the Responses API: the official DeepSeek endpoint
has a narrower documented custom-tool contract.

Existing registries remain readable without these new optional fields. Existing
chats retain their stored multi-agent runtime. Start a new V1 chat when changing
from V2 to V1.

## Personal V1 roles

Keep provider-specific role files under the actual `CODEX_HOME/agents` directory.
For example, `agents/deepseek_v1.toml`:

```toml
model = "DeepSeek-V4.1-Flash"
model_provider = "gateway"
model_reasoning_effort = "high"
model_catalog_json = "/absolute/path/to/CODEX_HOME/model-catalogs/gateway.json"
developer_instructions = "Always call view_image directly when it is provided as a top-level tool; never wrap that image call in exec. Use exec and ALL_TOOLS to discover and call deferred tools."
```

Register the role in `config.toml`:

```toml
[agents.deepseek_v1]
description = "DeepSeek through the configured gateway"
config_file = "agents/deepseek_v1.toml"
```

Create low and max roles if needed, changing only the reasoning effort. V1
children inherit the parent's base instructions, so keep the image guidance in
`developer_instructions` as well as the generated catalog. Delegate using
`agent_type="deepseek_v1"` and `fork_context=false`, with an explicit readable
handoff. OpenAI's encrypted history cannot be transferred to a different provider.

## Focused checks

From `codex-rs`, with the repository's Rust toolchain and `cargo-nextest`:

```sh
just test -p codex-cli --bin codex -E 'test(provider_cmd)'
just test -p codex-app-server -E 'test(model_list)'
```

Native acceptance checks should additionally cover repeated `provider apply`,
model listing under both first-party and external defaults, actual image
inspection, continuation of the same V1 child, waiting, and a child's
`send_input` to a normally completed parent. Human-interrupted parents and cold
resume require separate lifecycle tests.

Build and package both `codex` and `codex-code-mode-host` from the same revision.
Use `just assemble-codex-package --cargo-profile release`; the package builder
verifies the pinned Codex V8 artifacts. Switch the desktop's stable backend path
only after isolated acceptance checks pass, keeping a rollback package.
