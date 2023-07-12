use std::{future::IntoFuture, ops::Deref};

use anyhow::Result;
use auto_hash_map::AutoSet;
use futures::TryFutureExt;
use turbo_tasks::debug::ValueDebugFormat;

use crate::{self as turbo_tasks, RawVc, TryJoinIterExt, ValueToString, ValueToStringVc};

#[turbo_tasks::value(transparent)]
pub struct String(std::string::String);

#[turbo_tasks::value_impl]
impl StringVc {
    #[turbo_tasks::function]
    pub fn empty() -> Self {
        Self::cell("".to_string())
    }
}

#[turbo_tasks::value(transparent)]
pub struct OptionU16(Option<u16>);

#[turbo_tasks::value(transparent)]
pub struct U32(u32);

#[turbo_tasks::value(transparent)]
pub struct U64(u64);

#[turbo_tasks::value_impl]
impl ValueToString for U64 {
    #[turbo_tasks::function]
    fn to_string(&self) -> StringVc {
        StringVc::cell(self.0.to_string())
    }
}

#[turbo_tasks::value(transparent)]
pub struct OptionString(Option<std::string::String>);

#[turbo_tasks::value_impl]
impl OptionStringVc {
    #[turbo_tasks::function]
    pub fn none() -> Self {
        Self::cell(None)
    }
}

impl Default for OptionStringVc {
    fn default() -> Self {
        Self::none()
    }
}

#[turbo_tasks::value(transparent)]
pub struct Strings(Vec<std::string::String>);

#[turbo_tasks::value_impl]
impl StringsVc {
    #[turbo_tasks::function]
    pub fn empty() -> Self {
        Self::cell(Vec::new())
    }
}

#[turbo_tasks::value(transparent)]
pub struct Bool(bool);

#[turbo_tasks::value(transparent)]
pub struct Bools(Vec<bool>);

#[turbo_tasks::value(transparent)]
pub struct BoolVcs(Vec<BoolVc>);

#[turbo_tasks::value_impl]
impl BoolVcsVc {
    #[turbo_tasks::function]
    pub fn empty() -> Self {
        Self::cell(Vec::new())
    }

    #[turbo_tasks::function]
    async fn into_bools(self) -> Result<BoolsVc> {
        let this = self.await?;

        let bools = this
            .iter()
            .map(|b| b.into_future().map_ok(|b| *b))
            .try_join()
            .await?;

        Ok(BoolsVc::cell(bools))
    }

    #[turbo_tasks::function]
    pub async fn all(self) -> Result<BoolVc> {
        let bools = self.into_bools().await?;

        Ok(BoolVc::cell(bools.iter().all(|b| *b)))
    }

    #[turbo_tasks::function]
    pub async fn any(self) -> Result<BoolVc> {
        let bools = self.into_bools().await?;

        Ok(BoolVc::cell(bools.iter().any(|b| *b)))
    }
}

#[turbo_tasks::value(transparent)]
pub struct Usize(usize);

#[turbo_tasks::value(transparent)]
pub struct RawVcSet(AutoSet<RawVc>);

#[derive(ValueDebugFormat)]
#[turbo_tasks::value(transparent)]
pub struct JsonValue(pub serde_json::Value);

#[turbo_tasks::value_impl]
impl ValueToString for JsonValue {
    #[turbo_tasks::function]
    fn to_string(&self) -> StringVc {
        StringVc::cell(self.0.to_string())
    }
}

#[turbo_tasks::value(transparent, eq = "manual")]
#[derive(Debug, Clone)]
pub struct Regex(
    #[turbo_tasks(trace_ignore)]
    #[serde(with = "serde_regex")]
    pub regex::Regex,
);

impl Deref for Regex {
    type Target = regex::Regex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq for Regex {
    fn eq(&self, other: &Regex) -> bool {
        // Context: https://github.com/rust-lang/regex/issues/313#issuecomment-269898900
        self.0.as_str() == other.0.as_str()
    }
}
impl Eq for Regex {}
