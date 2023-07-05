use std::io::Write;

use anyhow::Result;
use indoc::writedoc;
use turbo_tasks::primitives::OptionStringVc;
use turbopack_core::{
    code_builder::{CodeBuilder, CodeVc},
    context::AssetContext,
    environment::{ChunkLoading, EnvironmentVc},
};
use turbopack_ecmascript::utils::StringifyJs;

use crate::{asset_context::get_runtime_asset_context, embed_js::embed_static_code};

/// Returns the code for the development ECMAScript runtime.
#[turbo_tasks::function]
pub async fn get_dev_runtime_code(
    environment: EnvironmentVc,
    chunk_base_path: OptionStringVc,
) -> Result<CodeVc> {
    let asset_context = get_runtime_asset_context(environment);

    let shared_runtime_utils_code = embed_static_code(asset_context, "shared/runtime-utils.ts");
    let runtime_base_code = embed_static_code(asset_context, "dev/runtime/base/runtime-base.ts");

    let chunk_loading = &*asset_context
        .compile_time_info()
        .environment()
        .chunk_loading()
        .await?;

    let runtime_backend_code = embed_static_code(
        asset_context,
        match chunk_loading {
            ChunkLoading::None => "dev/runtime/none/runtime-backend-none.ts",
            ChunkLoading::NodeJs => "dev/runtime/nodejs/runtime-backend-nodejs.ts",
            ChunkLoading::Dom => "dev/runtime/dom/runtime-backend-dom.ts",
        },
    );

    let mut code: CodeBuilder = CodeBuilder::default();

    writedoc!(
        code,
        r#"
            (() => {{
            if (!Array.isArray(globalThis.TURBOPACK)) {{
                return;
            }}

            const CHUNK_BASE_PATH = {};
        "#,
        StringifyJs(if let Some(chunk_base_path) = &*chunk_base_path.await? {
            chunk_base_path.as_str()
        } else {
            ""
        })
    )?;

    code.push_code(&*shared_runtime_utils_code.await?);
    code.push_code(&*runtime_base_code.await?);

    if matches!(chunk_loading, ChunkLoading::NodeJs) {
        code.push_code(&*embed_static_code(asset_context, "shared-node/require.ts").await?);
    }

    code.push_code(&*runtime_backend_code.await?);

    // Registering chunks depends on the BACKEND variable, which is set by the
    // specific runtime code, hence it must be appended after it.
    writedoc!(
        code,
        r#"
            const chunksToRegister = globalThis.TURBOPACK;
            globalThis.TURBOPACK = {{ push: registerChunk }};
            chunksToRegister.forEach(registerChunk);
            }})();
        "#
    )?;

    Ok(CodeVc::cell(code.build()))
}
