//! Preserve glTF metadata while extracting resources into a relocatable project folder.
use super::*;

pub struct ModelPackage {
    /// Generated flat filenames only. The document is always `model.gltf`.
    pub files: BTreeMap<String, Vec<u8>>,
}

/// Preserve model bytes and dependency names for runtime export. Unlike import conversion,
/// this keeps material/surface identities and baked-lighting fingerprints unchanged.
pub struct SourcePackage {
    pub primary: String,
    pub files: BTreeMap<String, Vec<u8>>,
}

pub fn package_model(path: &Path, progress: &job::Progress) -> Result<SourcePackage> {
    progress.stage("Reading model and dependencies")?;
    let snapshot = source_snapshot(path)?;
    let primary = path
        .file_name()
        .and_then(|p| p.to_str())
        .context("model filename is not UTF-8")?
        .to_owned();
    let mut files = BTreeMap::new();
    files.insert(
        primary.clone(),
        snapshot.primary.map_err(anyhow::Error::msg)?,
    );
    for (dependency, bytes) in snapshot.dependencies {
        progress.check()?;
        let relative = dependency.strip_prefix(path.parent().unwrap_or(Path::new(".")))?;
        ensure!(
            relative.components().all(|c| matches!(
                c,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )),
            "model dependency escapes export"
        );
        let name = relative
            .to_str()
            .context("model dependency is not UTF-8")?
            .replace('\\', "/");
        files.insert(name, bytes.map_err(anyhow::Error::msg)?);
    }
    Ok(SourcePackage { primary, files })
}

pub fn package_gltf(path: &Path, progress: &job::Progress) -> Result<ModelPackage> {
    progress.stage("Reading model resources")?;
    let snapshot = source_snapshot(path)?;
    let bytes = snapshot
        .primary
        .as_ref()
        .map_err(|e| anyhow::anyhow!(e.clone()))?;
    let gltf = gltf::Gltf::from_slice(bytes)?;
    let mut json = gltf_preflight(bytes)?;
    let mut files = BTreeMap::new();
    let mut uris = BTreeMap::<String, String>::new();
    for buffer in gltf.buffers() {
        progress.check()?;
        let key = match buffer.source() {
            gltf::buffer::Source::Bin => "glb:bin",
            gltf::buffer::Source::Uri(uri) => uri,
        };
        let name = if let Some(name) = uris.get(key) {
            name.clone()
        } else {
            let data = match buffer.source() {
                gltf::buffer::Source::Bin => gltf.blob.clone().context("missing GLB buffer")?,
                gltf::buffer::Source::Uri(uri) if uri.starts_with("data:") => data_uri(uri)?,
                gltf::buffer::Source::Uri(uri) => snapshot_resource(path, uri, &snapshot)?.to_vec(),
            };
            let name = format!("buffer-{}.bin", buffer.index());
            files.insert(name.clone(), data);
            uris.insert(key.to_owned(), name.clone());
            name
        };
        json["buffers"][buffer.index()]["uri"] = name.into();
    }
    for image in gltf.images() {
        progress.check()?;
        if let gltf::image::Source::Uri { uri, mime_type } = image.source() {
            let name = if let Some(name) = uris.get(uri) {
                name.clone()
            } else {
                let data = if uri.starts_with("data:") {
                    data_uri(uri)?
                } else {
                    snapshot_resource(path, uri, &snapshot)?.to_vec()
                };
                let mime = mime_type.unwrap_or_else(|| mime_for_uri(uri));
                let extension = if mime == "image/jpeg" || uri.starts_with("data:image/jpeg;") {
                    "jpg"
                } else {
                    "png"
                };
                let name = format!("image-{}.{}", image.index(), extension);
                files.insert(name.clone(), data);
                uris.insert(uri.to_owned(), name.clone());
                name
            };
            json["images"][image.index()]["uri"] = name.into();
        }
    }
    files.insert("model.gltf".into(), serde_json::to_vec(&json)?);
    Ok(ModelPackage { files })
}
