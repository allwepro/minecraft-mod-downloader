use crate::resource_downloader::business::xcache::{
    AnyCacheData, CacheContext, CacheType, CoreCacheManager, FetchFn,
};
use crate::resource_downloader::domain::{
    GameLoader, GameVersion, ProjectLnk, RTProjectData, RTProjectVersion, ResourceType,
};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct RTProjectPool {
    cache: Arc<CoreCacheManager>,
}

impl RTProjectPool {
    pub fn new(cache: Arc<CoreCacheManager>) -> Self {
        Self { cache }
    }

    /// Searches for projects matching the given query
    pub fn search(
        &self,
        query: String,
        resource_type: ResourceType,
        version: Option<GameVersion>,
        loader: Option<GameLoader>,
    ) -> anyhow::Result<Option<Vec<ProjectLnk>>> {
        let search_ctx = CacheContext {
            // Used a pseudo-project link to represent the query in the hash - not pretty but functional
            id: Some(query.clone()),
            resource_type: Some(resource_type),
            version: version.clone(),
            loader: loader.clone(),
        };

        let res = self.cache.get::<Vec<ProjectLnk>>(
            CacheType::Search,
            search_ctx.clone(),
            Box::new(move |p_ctx| {
                Box::pin(async move {
                    let results = p_ctx
                        .provider
                        .search_projects(
                            &p_ctx,
                            query,
                            &resource_type,
                            version.as_ref(),
                            loader.as_ref(),
                        )
                        .await?;
                    Ok(Arc::new(results) as AnyCacheData)
                })
            }),
        )?;

        Ok(res.map(|links| links.into_iter().collect()))
    }

    /// Fetches the specific project for a slug
    pub async fn get_project_by_slug_blocking(
        &self,
        slug: String,
        resource_type: ResourceType,
    ) -> anyhow::Result<Option<ProjectLnk>> {
        let ctx = CacheContext {
            id: Some(slug.clone()),
            resource_type: None,
            version: None,
            loader: None,
        };

        self.cache
            .get_blocking::<ProjectLnk>(
                CacheType::ProjectSlug,
                ctx,
                Box::new(move |p_ctx| {
                    Box::pin(async move {
                        let data = p_ctx
                            .provider
                            .fetch_project_from_slug(&p_ctx, slug, &resource_type, None, None)
                            .await?;
                        Ok(Arc::new(data) as AnyCacheData)
                    })
                }),
                Duration::from_secs(5),
            )
            .await
    }

    /// Fetches project metadata (Icon URL, Description, etc.)
    pub fn get_metadata(
        &self,
        project: ProjectLnk,
        resource_type: ResourceType,
    ) -> anyhow::Result<Option<RTProjectData>> {
        let (ctx, fun) = Self::metadata_prepare_request(&project, &resource_type);

        self.cache
            .get::<RTProjectData>(CacheType::ProjectMetadata, ctx, fun)
    }

    pub async fn get_metadata_blocking(
        &self,
        project: ProjectLnk,
        resource_type: ResourceType,
    ) -> anyhow::Result<Option<RTProjectData>> {
        let (ctx, fun) = Self::metadata_prepare_request(&project, &resource_type);

        self.cache
            .get_blocking::<RTProjectData>(
                CacheType::ProjectMetadata,
                ctx,
                fun,
                Duration::from_secs(5),
            )
            .await
    }

    pub fn clear_metadata(&self, project: ProjectLnk, resource_type: ResourceType) {
        let (ctx, _) = Self::metadata_prepare_request(&project, &resource_type);
        self.cache.discard(CacheType::ProjectMetadata, ctx);
    }

    /// Fetches the list of versions/files for a specific game version and loader
    pub fn get_versions(
        &self,
        project: ProjectLnk,
        resource_type: ResourceType,
        version: GameVersion,
        loader: GameLoader,
    ) -> anyhow::Result<Option<Vec<RTProjectVersion>>> {
        let (ctx, fun) = Self::versions_prepare_request(
            &project,
            resource_type,
            version.clone(),
            loader.clone(),
        );

        self.cache
            .get::<Vec<RTProjectVersion>>(CacheType::ProjectVersions, ctx, fun)
    }
    pub async fn get_versions_blocking(
        &self,
        project: ProjectLnk,
        resource_type: ResourceType,
        version: GameVersion,
        loader: GameLoader,
    ) -> anyhow::Result<Option<Vec<RTProjectVersion>>> {
        let (ctx, fun) = Self::versions_prepare_request(
            &project,
            resource_type,
            version.clone(),
            loader.clone(),
        );

        self.cache
            .get_blocking::<Vec<RTProjectVersion>>(
                CacheType::ProjectVersions,
                ctx,
                fun,
                Duration::from_secs(5),
            )
            .await
    }

    /// Warms the slug cache for a specific project
    pub fn warm_slug(&self, slug: String, _resource_type: ResourceType, project: ProjectLnk) {
        let ctx = CacheContext {
            id: Some(slug),
            resource_type: None,
            version: None,
            loader: None,
        };

        self.cache.warm(
            CacheType::ProjectSlug,
            ctx,
            Arc::new(project.to_context_id()) as AnyCacheData,
        );
    }

    /// Warms the metadata cache for a specific project
    pub fn warm_metadata(
        &self,
        project: ProjectLnk,
        resource_type: ResourceType,
        data: RTProjectData,
    ) {
        let ctx = CacheContext {
            id: project.to_context_id(),
            resource_type: Some(resource_type),
            version: None,
            loader: None,
        };

        self.cache.warm(
            CacheType::ProjectMetadata,
            ctx,
            Arc::new(data) as AnyCacheData,
        );
    }

    // ------- Helper to prepare requests -----
    fn metadata_prepare_request(
        project: &ProjectLnk,
        resource_type: &ResourceType,
    ) -> (CacheContext, FetchFn) {
        let project_owned = project.clone();
        let resource_type_owned = *resource_type;
        (
            CacheContext {
                id: project.to_context_id(),
                resource_type: Some(*resource_type),
                version: None,
                loader: None,
            },
            Box::new(move |p_ctx| {
                let project = project_owned;
                let resource_type = resource_type_owned;
                Box::pin(async move {
                    let data = p_ctx
                        .provider
                        .fetch_project_data(&p_ctx, project, &resource_type, None, None)
                        .await?;
                    Ok(Arc::new(data) as AnyCacheData)
                })
            }),
        )
    }

    fn versions_prepare_request(
        project: &ProjectLnk,
        resource_type: ResourceType,
        version: GameVersion,
        loader: GameLoader,
    ) -> (CacheContext, FetchFn) {
        let project_owned = project.clone();
        let resource_type_owned = resource_type;
        let version_owned = version.clone();
        let loader_owned = loader.clone();
        (
            CacheContext {
                id: project.to_context_id(),
                resource_type: Some(resource_type),
                version: Some(version.clone()),
                loader: Some(loader.clone()),
            },
            Box::new(move |p_ctx| {
                let project = project_owned.clone();
                let resource_type = resource_type_owned;
                let version = version_owned.clone();
                let loader = loader_owned.clone();
                Box::pin(async move {
                    let data = p_ctx
                        .provider
                        .fetch_project_versions(
                            &p_ctx,
                            project,
                            &resource_type,
                            Some(&version),
                            Some(&loader),
                        )
                        .await?;
                    Ok(Arc::new(data) as AnyCacheData)
                })
            }),
        )
    }
}
