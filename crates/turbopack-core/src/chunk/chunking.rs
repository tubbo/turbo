use std::collections::HashSet;

use anyhow::{bail, Result};
use turbo_tasks::{
    graph::{GraphTraversal, ReverseTopological},
    Value, ValueToString,
};

use super::{
    availability_info::AvailabilityInfo, ChunkGroupReferenceVc, ChunkItemVc,
    ChunkableAssetReference, ChunkableAssetReferenceVc, ChunkingContextVc,
};
use crate::{
    asset::{Asset, AssetVc, AssetsVc},
    chunk::{ChunkItem, ChunkableAsset, ChunkableAssetVc, ChunkingType},
    reference::{AssetReference, AssetReferenceVc},
};

#[turbo_tasks::value]
struct ChunkingResult {
    chunk_items: Vec<ChunkItemVc>,
    available_assets: HashSet<AssetVc>,
    isolated_parallel_chunk_groups: Vec<AssetVc>,
    external_references: Vec<AssetReferenceVc>,
    async_assets: Vec<AssetVc>,
}

#[turbo_tasks::function]
async fn chunking(
    entries: Vec<ChunkableAssetVc>,
    context: ChunkingContextVc,
    availability_info: Value<AvailabilityInfo>,
) -> Result<ChunkingResultVc> {
    #[derive(Clone, PartialEq, Eq, Hash)]
    enum ResultItem {
        ChunkItem(ChunkItemVc, AssetVc),
        External(AssetReferenceVc),
        IsolatedParallel(AssetVc),
        Async(AssetVc),
    }
    let roots = entries.iter().map(|&asset| {
        ResultItem::ChunkItem(
            asset.as_chunk_item(context, availability_info),
            asset.into(),
        )
    });
    let results = ReverseTopological::new()
        .skip_duplicates()
        .visit(roots, |result: &ResultItem| {
          let chunk_item = if let &ResultItem::ChunkItem(chunk_item, _) = result {
            Some(chunk_item)
          } else {
            None
          };
          async move {
              let Some(chunk_item) = chunk_item else {
                  return Ok(Vec::new());
              };
              let mut results = Vec::new();
              for &reference in chunk_item.references().await?.iter() {
                  if let Some(chunkable) = ChunkableAssetReferenceVc::resolve_from(reference).await? {
                      match &*chunkable.chunking_type().await? {
                          None => results.push(ResultItem::External(reference)),
                          Some(
                              ChunkingType::Parallel
                              | ChunkingType::PlacedOrParallel
                              | ChunkingType::Placed,
                          ) => {
                              for &asset in &*chunkable.resolve_reference().primary_assets().await? {
                                  let Some(chunkable) = ChunkableAssetVc::resolve_from(asset).await? else {
                                      bail!(
                                          "asset {} must be a ChunkableAsset when it's referenced from a ChunkableAssetReference",
                                          asset.ident().to_string().await?
                                      );
                                  };
                                  results.push(ResultItem::ChunkItem(
                                      chunkable.as_chunk_item(context, availability_info),
                                      chunkable.into(),
                                  ));
                              }
                          }
                          Some(ChunkingType::IsolatedParallel) => {
                              for &asset in &*chunkable.resolve_reference().primary_assets().await? {
                                  results.push(ResultItem::IsolatedParallel(asset));
                              }
                          }
                          Some(ChunkingType::Async) => {
                              for &asset in &*chunkable.resolve_reference().primary_assets().await? {
                                  results.push(ResultItem::Async(asset));
                              }
                          }
                      }
                  } else {
                      results.push(ResultItem::External(reference));
                  }
              }
              Ok(results)
          }
      })
        .await
        .completed()?;

    let mut chunk_items = Vec::new();
    let mut isolated_parallel_chunk_groups = Vec::new();
    let mut external_references = Vec::new();
    let mut async_assets = Vec::new();
    let mut available_assets = HashSet::new();
    for item in results.into_inner().into_iter() {
        match item {
            ResultItem::ChunkItem(chunk_item, asset) => {
                chunk_items.push(chunk_item);
                available_assets.insert(asset);
            }
            ResultItem::External(reference) => {
                external_references.push(reference);
            }
            ResultItem::IsolatedParallel(asset) => {
                isolated_parallel_chunk_groups.push(asset);
            }
            ResultItem::Async(asset) => {
                async_assets.push(asset);
            }
        }
    }
    Ok(ChunkingResult {
        chunk_items,
        available_assets,
        isolated_parallel_chunk_groups,
        external_references,
        async_assets,
    }
    .cell())
}

/// Computes the chunks for a chunk group defined by a list of entries in a
/// specific context and with some availability info. The returned chunks are
/// optimized based on the optimization ability of the `context`.
async fn chunk_group(
    entries: Vec<ChunkableAssetVc>,
    context: ChunkingContextVc,
    availability_info: Value<AvailabilityInfo>,
) -> Result<AssetsVc> {
    // Capture all chunk items and other things from the module graph
    let chunking_result = chunking(entries, context, availability_info).await?;

    // Get innner availablity info
    let inner_availability_info = todo!();

    // Additional references from the main chunk
    let mut inner_references = Vec::new();

    // Async chunk groups
    for &async_chunk_group in chunking_result.async_assets.iter() {
        inner_references.push(AsyncChunkGroupReferenceVc::new(
            context,
            async_chunk_group,
            inner_availability_info,
        ));
    }

    // Separate chunk groups
    for &async_chunk_group in chunking_result.async_assets.iter() {
        inner_references.push(ChunkGroupReferenceVc::new(
            context,
            async_chunk_group,
            inner_availability_info,
        ));
    }

    // Place chunk items in chunks in a smart way
    let chunks: Vec<AssetVc> = todo!("passing in inner_references");

    // merge parallel chunk groups
    for chunk_group in todo!("recursive with isolated_parallel_chunk_groups").await? {
        chunks.extend(chunk_group)
    }
}
