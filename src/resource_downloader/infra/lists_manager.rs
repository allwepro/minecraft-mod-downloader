use crate::resource_downloader::domain::{ListLnk, ProjectList as ListFile, ProjectList};
use anyhow::{Context, Result};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Mutex;

pub struct ListFileManager {
    lists_dir: PathBuf,
    id_filename_map: Arc<Mutex<HashMap<ListLnk, String>>>,
    locks: Arc<RwLock<HashMap<ListLnk, Arc<Mutex<()>>>>>,
}

#[allow(dead_code)]
impl ListFileManager {
    pub fn new(lists_dir: PathBuf) -> Self {
        Self {
            lists_dir,
            id_filename_map: Arc::new(Mutex::new(HashMap::new())),
            locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn primitive_get_list_path(&self, filename: &str) -> PathBuf {
        self.lists_dir.join(format!("{filename}.mmd"))
    }

    // id -> path+filename mapping
    pub async fn init(&self) -> Result<()> {
        let mut list_filenames = Vec::new();

        if !self.lists_dir.exists() {
            return Ok(());
        }

        let mut dir = fs::read_dir(&self.lists_dir).await?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("mmd")
                && let Some(file_stem) = path.file_stem().and_then(|s| s.to_str())
            {
                list_filenames.push(file_stem.to_string());
            }
        }

        for filename in list_filenames {
            let path = self.primitive_get_list_path(&filename);
            if path.exists() {
                let content = fs::read_to_string(&path)
                    .await
                    .context(format!("Failed to read list file: {}", path.display()))?;
                let list_file: ListFile = toml::from_str(&content)
                    .context(format!("Failed to parse list file: {}", path.display()))?;
                self.internal_set_filename_cache(
                    &ListLnk::new(list_file.get_id()),
                    filename.clone(),
                )
                .await;
            }
        }
        Ok(())
    }
    async fn internal_get_filename_cache(&self, list: &ListLnk) -> Option<String> {
        let map = self.id_filename_map.lock().await;
        map.get(list).cloned()
    }

    async fn internal_set_filename_cache(&self, list: &ListLnk, filename: String) {
        let mut map = self.id_filename_map.lock().await;
        map.insert(list.clone(), filename);
    }

    async fn internal_remove_filename_cache(&self, list: &ListLnk) {
        let mut map = self.id_filename_map.lock().await;
        map.remove(list);
    }

    async fn get_available_lists(&self) -> Vec<ListLnk> {
        let map = self.id_filename_map.lock().await;
        map.keys().cloned().collect()
    }

    async fn get_list_path(&self, list: &ListLnk) -> Option<PathBuf> {
        self.internal_get_filename_cache(list)
            .await
            .map(|filename| self.primitive_get_list_path(&filename))
    }

    async fn get_lock(&self, list: &ListLnk) -> Arc<Mutex<()>> {
        let mut locks = self.locks.write();
        locks
            .entry(list.clone())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    // general operations
    pub async fn has_any_lists(&self) -> Result<bool> {
        Ok(self.list_count().await? > 0)
    }

    pub async fn list_count(&self) -> Result<usize> {
        let list = self.id_filename_map.lock().await;
        Ok(list.len())
    }

    pub async fn exists(&self, list: &ListLnk) -> bool {
        self.get_list_path(list)
            .await
            .is_some_and(|path| path.exists())
    }

    pub async fn load_all(&self) -> Result<Vec<ListFile>> {
        let mut lists = Vec::new();
        for id in self.get_available_lists().await {
            lists.push(self.load(&id).await?);
        }
        Ok(lists)
    }

    pub async fn load(&self, list: &ListLnk) -> Result<ListFile> {
        let lock = self.get_lock(list).await;
        let _guard = lock.lock().await;

        let path = self.get_list_path(list).await;
        if path.is_none() {
            return Err(anyhow::anyhow!("List file not found for list ID: {list}"));
        }
        let path_target = path.unwrap();

        let list_file = fs::read_to_string(&path_target).await.context(format!(
            "Failed to read list file: {}",
            path_target.display()
        ))?;

        let project_list: ListFile = toml::from_str(&list_file).context(format!(
            "Failed to parse list file: {}",
            path_target.display()
        ))?;

        Ok(project_list)
    }

    pub async fn save(&self, target_list: &ListFile) -> Result<()> {
        let list = target_list.get_lnk();
        let lock = self.get_lock(&list).await;
        let _guard = lock.lock().await;

        let path_target = match self.get_list_path(&list).await {
            Some(p) => p,
            None => {
                let name = target_list.get_id();
                self.internal_set_filename_cache(&list, name.clone()).await;
                self.primitive_get_list_path(&name)
            }
        };

        if let Some(parent) = path_target.parent() {
            fs::create_dir_all(parent).await?;
        }

        let toml_str =
            toml::to_string_pretty(target_list).context("Failed to serialize list file")?;

        let temp_path = path_target.with_extension("mmd.tmp");

        fs::write(&temp_path, toml_str).await?;
        fs::rename(&temp_path, &path_target).await?;

        Ok(())
    }

    pub async fn save_raw(&self, list: &ListLnk, content: String) -> Result<()> {
        let lock = self.get_lock(list).await;
        let _guard = lock.lock().await;

        let path_target = match self.get_list_path(list).await {
            Some(p) => p,
            None => {
                let name = list.to_string();
                self.internal_set_filename_cache(list, name.clone()).await;
                self.primitive_get_list_path(&name)
            }
        };

        if let Some(parent) = path_target.parent() {
            fs::create_dir_all(parent).await?;
        }

        let temp_path = path_target.with_extension("mmd.tmp");
        fs::write(&temp_path, content).await?;
        fs::rename(&temp_path, &path_target).await?;

        Ok(())
    }

    pub async fn delete(&self, list: &ListLnk) -> Result<()> {
        let lock = self.get_lock(list).await;
        let _guard = lock.lock().await;

        let path = self.get_list_path(list).await;
        if path.is_none() {
            return Err(anyhow::anyhow!("List file not found for list ID: {list}"));
        }
        let path_target = path.unwrap();

        if path_target.exists() {
            fs::remove_file(&path_target).await.context(format!(
                "Failed to delete list file: {}",
                path_target.display()
            ))?;
        }

        drop(_guard);

        {
            let mut locks = self.locks.write();
            locks.remove(list);
        }

        self.internal_remove_filename_cache(list).await;

        Ok(())
    }

    pub async fn delete_multiple(&self, lists: &[ListLnk]) -> Result<()> {
        let mut failed = Vec::new();

        for list in lists {
            if self.delete(list).await.is_err() {
                failed.push(list.clone());
            }
        }

        if !failed.is_empty() {
            return Err(anyhow::anyhow!("Failed to delete lists: {failed:?}"));
        }

        Ok(())
    }

    pub async fn modify<F, R>(&self, list: &ListLnk, f: F) -> Result<R>
    where
        F: FnOnce(&mut ListFile) -> R,
    {
        let mut project_list = self.load(list).await?;

        let result = f(&mut project_list);

        self.save(&project_list).await?;

        Ok(result)
    }

    pub async fn modify_async<F, Fut, R>(&self, list: &ListLnk, f: F) -> Result<R>
    where
        F: FnOnce(&mut ListFile) -> Fut,
        Fut: Future<Output = R>,
    {
        let mut project_list = self.load(list).await?;

        let result = f(&mut project_list).await;

        self.save(&project_list).await?;

        Ok(result)
    }

    pub async fn validate(&self, list: &ListLnk) -> Result<bool> {
        match self.load(list).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    pub async fn copy(&self, list: &ListLnk) -> Result<ListFile> {
        let mut target_list = self.load(list).await?;
        target_list = ListFile::new_from_existing(&target_list, ProjectList::generate_id());
        self.save(&target_list).await?;
        Ok(target_list)
    }

    pub async fn import_from_string(&self, content: &str) -> Result<ListFile> {
        let mut list: ListFile = toml::from_str(content)?;
        list = ListFile::new_from_existing(&list, ProjectList::generate_id());
        self.save(&list).await?;
        Ok(list)
    }

    pub async fn export_to_string(&self, list: &ListLnk) -> Result<String> {
        let list_str = self.load(list).await?;
        Ok(toml::to_string_pretty(&list_str)?)
    }

    pub async fn import_from_file(&self, path: PathBuf) -> Result<ListFile> {
        let content = fs::read_to_string(&path).await?;
        self.import_from_string(&content).await
    }

    pub async fn export_to_file(&self, list: &ListLnk, path: PathBuf) -> Result<()> {
        let content = self.export_to_string(list).await?;
        fs::write(&path, content).await?;
        Ok(())
    }
}
