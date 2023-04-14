use anyhow::Result;
use turbo_tasks::Vc;
use turbo_tasks_fs::FileContent;
use turbopack_core::{
    asset::{Asset, AssetContent},
    ident::AssetIdent,
};

use crate::utils::StringifyJs;

#[turbo_tasks::function]
fn modifier() -> Vc<String> {
    Vc::cell("text content".to_string())
}

/// A source asset that exports the string content of an asset as the default
/// export of a JS module.
#[turbo_tasks::value]
pub struct TextContentSourceAsset {
    pub source: Vc<Box<dyn Asset>>,
}

#[turbo_tasks::value_impl]
impl TextContentSourceAsset {
    #[turbo_tasks::function]
    pub fn new(source: Vc<Box<dyn Asset>>) -> Vc<Self> {
        TextContentSourceAsset { source }.cell()
    }
}

#[turbo_tasks::value_impl]
impl Asset for TextContentSourceAsset {
    #[turbo_tasks::function]
    fn ident(&self) -> Vc<AssetIdent> {
        self.source
            .ident()
            .with_modifier(modifier())
            .rename_as("*.mjs".to_string())
    }

    #[turbo_tasks::function]
    async fn content(&self) -> Result<Vc<AssetContent>> {
        let source = self.source.content().file_content();
        let FileContent::Content(content) = &*source.await? else {
            return Ok(AssetContent::file(FileContent::NotFound.cell()));
        };
        let text = content.content().to_str()?;
        let code = format!("export default {};", StringifyJs(&text));
        let content = FileContent::Content(code.into()).cell();
        Ok(AssetContent::file(content))
    }
}
