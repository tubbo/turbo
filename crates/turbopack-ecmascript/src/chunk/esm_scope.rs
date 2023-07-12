use std::collections::HashMap;

use anyhow::{Context, Result};
use petgraph::{algo::tarjan_scc, prelude::DiGraphMap};
use turbo_tasks::{TryFlatJoinIterExt, Value};
use turbopack_core::{
    asset::{Asset, AssetsSetVc},
    chunk::{availability_info::AvailabilityInfo, available_assets::chunkable_assets_set},
};

use crate::{
    chunk::EcmascriptChunkPlaceableVc,
    references::esm::{base::ReferencedAsset, EsmAssetReferenceVc},
    EcmascriptModuleAssetVc, EcmascriptModuleAssetsVc,
};

/// A graph representing all ESM imports in a chunk group.
#[turbo_tasks::value(serialization = "none", cell = "new", eq = "manual")]
pub(crate) struct EsmScope {
    scc_map: HashMap<EcmascriptChunkPlaceableVc, EsmScopeSccVc>,
    #[turbo_tasks(trace_ignore, debug_ignore)]
    scc_graph: DiGraphMap<EsmScopeSccVc, ()>,
}

#[turbo_tasks::value(transparent)]
pub(crate) struct EsmScopeScc(Vec<EcmascriptChunkPlaceableVc>);

#[turbo_tasks::value(transparent)]
pub(crate) struct OptionEsmScopeScc(Option<EsmScopeSccVc>);

#[turbo_tasks::value(transparent)]
pub(crate) struct EsmScopeSccs(Vec<EsmScopeSccVc>);

#[turbo_tasks::value_impl]
impl EsmScopeVc {
    #[turbo_tasks::function]
    pub(crate) async fn new(availability_info: Value<AvailabilityInfo>) -> Result<Self> {
        let assets = if let Some(root) = availability_info.current_availability_root() {
            chunkable_assets_set(root)
        } else {
            AssetsSetVc::empty()
        };

        let esm_assets = get_ecmascript_module_assets(assets);
        let import_references = collect_import_references(esm_assets).await?;

        let mut graph = DiGraphMap::new();

        for (parent, child) in &*import_references {
            graph.add_edge(*parent, *child, ());
        }

        let sccs = tarjan_scc(&graph);

        let mut scc_map = HashMap::new();
        for scc in sccs {
            let scc_vc = EsmScopeScc(scc.clone()).cell();

            for placeable in scc {
                scc_map.insert(placeable, scc_vc);
            }
        }

        let mut scc_graph = DiGraphMap::new();
        for (parent, child, _) in graph.all_edges() {
            let parent_scc_vc = *scc_map
                .get(&parent)
                .context("unexpected missing SCC in map")?;
            let child_scc_vc = *scc_map
                .get(&child)
                .context("unexpected missing SCC in map")?;

            if parent_scc_vc != child_scc_vc {
                scc_graph.add_edge(parent_scc_vc, child_scc_vc, ());
            }
        }

        Ok(Self::cell(EsmScope { scc_map, scc_graph }))
    }

    #[turbo_tasks::function]
    pub(crate) async fn get_scc(
        self,
        placeable: EcmascriptChunkPlaceableVc,
    ) -> Result<OptionEsmScopeSccVc> {
        let this = self.await?;

        Ok(OptionEsmScopeSccVc::cell(
            this.scc_map.get(&placeable).copied(),
        ))
    }

    #[turbo_tasks::function]
    pub(crate) async fn get_scc_children(self, scc: EsmScopeSccVc) -> Result<EsmScopeSccsVc> {
        let this = self.await?;

        let children = this.scc_graph.neighbors(scc).collect();

        Ok(EsmScopeSccsVc::cell(children))
    }
}

#[turbo_tasks::function]
async fn get_ecmascript_module_assets(assets: AssetsSetVc) -> Result<EcmascriptModuleAssetsVc> {
    let esm_assets = assets
        .await?
        .iter()
        .copied()
        .map(|r| async move { anyhow::Ok(EcmascriptModuleAssetVc::resolve_from(r).await?) })
        .try_flat_join()
        .await?;

    Ok(EcmascriptModuleAssetsVc::cell(esm_assets))
}

#[turbo_tasks::value(transparent)]
struct ImportReferences(Vec<(EcmascriptChunkPlaceableVc, EcmascriptChunkPlaceableVc)>);

#[turbo_tasks::function]
async fn collect_import_references(
    esm_assets: EcmascriptModuleAssetsVc,
) -> Result<ImportReferencesVc> {
    let import_references = esm_assets
        .await?
        .iter()
        .copied()
        .map(|a| async move {
            let placeable = a.as_ecmascript_chunk_placeable().resolve().await?;

            a.references()
                .await?
                .iter()
                .copied()
                .map(|r| async move {
                    let Some(r) = EsmAssetReferenceVc::resolve_from(r).await? else {
                        return Ok(None);
                    };

                    let ReferencedAsset::Some(child_placeable) = &*r.get_referenced_asset().await?
                    else {
                        return Ok(None);
                    };

                    let child_placeable = child_placeable.resolve().await?;

                    anyhow::Ok(Some((placeable, child_placeable)))
                })
                .try_flat_join()
                .await
        })
        .try_flat_join()
        .await?;

    Ok(ImportReferencesVc::cell(import_references))
}
