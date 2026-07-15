#![allow(clippy::needless_pass_by_value)]

use std::{
    fs,
    io::{Cursor, Write},
    path::{Path, PathBuf},
};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};
use serde::Serialize;
use tauri::State;

use atelier_app::CommandResult;
use atelier_app_api::{
    error::ErrorEnvelopeDto,
    resource::{
        GetResourceImageRequestDto, ImageResourceKindDto, ImportImageResourceRequestDto,
        ImportImageResourceResponseDto, ReleaseImportedImageResourcesRequestDto,
        SaveResourceImageRequestDto, SaveResourceImagesZipRequestDto,
    },
    vibe::{
        ExportVibeDocumentRequestDto, ImportEmbeddedPngVibeDocumentRequestDto,
        ImportVibeDocumentRequestDto, ImportedVibeDocumentsDto,
    },
    workspace::{
        AppBootstrapDto, CloseWorkspaceResponseDto, OpenWorkspaceRequestDto, WorkspaceStatusDto,
    },
};

use crate::{
    desktop::{DesktopState, TauriDialog, TauriPathOpener},
    desktop_system::{DesktopPaths, DesktopSystemError, DesktopSystemResult, PickFilesOptions},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedDesktopFileDto {
    pub path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedDesktopArchiveDto {
    pub path: PathBuf,
    pub exported: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardImageDto {
    pub image_base64: String,
    pub mime_type: &'static str,
}

#[tauri::command]
pub fn read_clipboard_image() -> CommandResult<ClipboardImageDto> {
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|error| ErrorEnvelopeDto::new("clipboard_unavailable", error.to_string()))?;
    let image = clipboard
        .get_image()
        .map_err(|error| ErrorEnvelopeDto::new("clipboard_image_unavailable", error.to_string()))?;
    let width = u32::try_from(image.width)
        .map_err(|error| ErrorEnvelopeDto::new("clipboard_image_invalid", error.to_string()))?;
    let height = u32::try_from(image.height)
        .map_err(|error| ErrorEnvelopeDto::new("clipboard_image_invalid", error.to_string()))?;
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&image.bytes, width, height, ExtendedColorType::Rgba8)
        .map_err(|error| ErrorEnvelopeDto::new("clipboard_image_encode", error.to_string()))?;
    Ok(ClipboardImageDto {
        image_base64: STANDARD.encode(png),
        mime_type: "image/png",
    })
}

#[tauri::command]
pub fn desktop_paths(state: State<'_, DesktopState>) -> DesktopPaths {
    state.system.paths().clone()
}

#[tauri::command]
pub fn pick_workspace_directory(state: State<'_, DesktopState>) -> CommandResult<Option<PathBuf>> {
    let dialog = TauriDialog::new(state.app_handle.clone());
    desktop_result(state.system.pick_workspace_directory(&dialog))
}

#[tauri::command]
pub fn pick_export_directory(state: State<'_, DesktopState>) -> CommandResult<Option<PathBuf>> {
    let dialog = TauriDialog::new(state.app_handle.clone());
    desktop_result(state.system.pick_export_directory(&dialog))
}

#[tauri::command]
pub async fn pick_and_import_image_resources(
    state: State<'_, DesktopState>,
    kind: ImageResourceKindDto,
    options: PickFilesOptions,
) -> CommandResult<Vec<ImportImageResourceResponseDto>> {
    let dialog = TauriDialog::new(state.app_handle.clone());
    let files = desktop_result(state.system.pick_image_files(&dialog, options))?;
    let requests = desktop_result(
        files
            .iter()
            .map(|path| image_resource_request_from_file(path, kind))
            .collect::<DesktopSystemResult<Vec<_>>>(),
    )?;
    let mut imported = Vec::with_capacity(requests.len());

    for request in requests {
        match state.host.import_image_resource(request).await {
            Ok(resource) => imported.push(resource),
            Err(error) => {
                let resources = imported.iter().map(|item| item.resource.clone()).collect();
                let _cleanup_result = state
                    .host
                    .release_imported_image_resources(ReleaseImportedImageResourcesRequestDto {
                        resources,
                    })
                    .await;
                return Err(error);
            }
        }
    }

    Ok(imported)
}

#[tauri::command]
pub async fn pick_and_import_vibe_documents(
    state: State<'_, DesktopState>,
    options: PickFilesOptions,
) -> CommandResult<ImportedVibeDocumentsDto> {
    let dialog = TauriDialog::new(state.app_handle.clone());
    let files = desktop_result(state.system.pick_vibe_documents(&dialog, options))?;
    let requests = desktop_result(
        files
            .iter()
            .map(|path| vibe_document_request_from_file(path))
            .collect::<DesktopSystemResult<Vec<_>>>(),
    )?;
    let mut entries = Vec::new();

    for request in requests {
        let imported = state.host.import_vibe_document(request).await?;
        entries.extend(imported.entries);
    }

    Ok(ImportedVibeDocumentsDto { entries })
}

#[tauri::command]
pub async fn pick_and_import_embedded_png_vibe_documents(
    state: State<'_, DesktopState>,
    options: PickFilesOptions,
) -> CommandResult<ImportedVibeDocumentsDto> {
    let dialog = TauriDialog::new(state.app_handle.clone());
    let files = desktop_result(state.system.pick_png_files(&dialog, options))?;
    let requests = desktop_result(
        files
            .iter()
            .map(|path| embedded_png_vibe_document_request_from_file(path))
            .collect::<DesktopSystemResult<Vec<_>>>(),
    )?;
    let mut entries = Vec::new();

    for request in requests {
        let imported = state
            .host
            .import_embedded_png_vibe_document(request)
            .await?;
        entries.extend(imported.entries);
    }

    Ok(ImportedVibeDocumentsDto { entries })
}

#[tauri::command]
pub async fn save_vibe_document(
    state: State<'_, DesktopState>,
    request: ExportVibeDocumentRequestDto,
) -> CommandResult<Option<SavedDesktopFileDto>> {
    let exported = state.host.export_vibe_document(request).await?;
    let dialog = TauriDialog::new(state.app_handle.clone());
    let default_file_name = format!("vibes.{}", exported.file_extension);
    let Some(path) =
        desktop_result(dialog.save_file(Some(&default_file_name), Some(&exported.file_extension)))?
    else {
        return Ok(None);
    };

    desktop_result(write_file_text(&path, &exported.content))?;
    desktop_result(state.system.allow_user_path(&path))?;
    Ok(Some(SavedDesktopFileDto { path }))
}

#[tauri::command]
pub async fn save_resource_image(
    state: State<'_, DesktopState>,
    request: SaveResourceImageRequestDto,
) -> CommandResult<Option<SavedDesktopFileDto>> {
    let image = state
        .host
        .get_resource_image(GetResourceImageRequestDto {
            resource: request.resource,
        })
        .await?;
    let extension = image_extension(image.mime_type.as_deref());
    let default_file_name = resource_file_name(request.suggested_file_name, extension);
    let dialog = TauriDialog::new(state.app_handle.clone());
    let Some(path) = desktop_result(dialog.save_file(Some(&default_file_name), Some(extension)))?
    else {
        return Ok(None);
    };
    let bytes = STANDARD
        .decode(image.image_base64.trim())
        .map_err(|error| ErrorEnvelopeDto::new("resource_decode_error", error.to_string()))?;

    desktop_result(write_file_bytes(&path, &bytes))?;
    desktop_result(state.system.allow_user_path(&path))?;
    Ok(Some(SavedDesktopFileDto { path }))
}

#[tauri::command]
pub async fn save_resource_images_zip(
    state: State<'_, DesktopState>,
    request: SaveResourceImagesZipRequestDto,
) -> CommandResult<Option<SavedDesktopArchiveDto>> {
    if request.entries.is_empty() {
        return Err(ErrorEnvelopeDto::new(
            "invalid_request",
            "at least one image is required for ZIP export",
        ));
    }
    let default_file_name = zip_file_name(request.suggested_file_name);
    let dialog = TauriDialog::new(state.app_handle.clone());
    let Some(path) = desktop_result(dialog.save_file(Some(&default_file_name), Some("zip")))?
    else {
        return Ok(None);
    };

    let exported = request.entries.len();
    let mut images = Vec::with_capacity(exported);
    for entry in request.entries {
        let image = state
            .host
            .get_resource_image(GetResourceImageRequestDto {
                resource: entry.resource,
            })
            .await?;
        let extension = image_extension(image.mime_type.as_deref());
        let file_name =
            resource_file_name(Some(sanitize_archive_name(&entry.file_name)), extension);
        let bytes = STANDARD
            .decode(image.image_base64.trim())
            .map_err(|error| ErrorEnvelopeDto::new("resource_decode_error", error.to_string()))?;
        images.push((file_name, bytes));
    }
    let archive = build_resource_images_zip(images)?;
    desktop_result(write_file_bytes(&path, &archive))?;
    desktop_result(state.system.allow_user_path(&path))?;
    Ok(Some(SavedDesktopArchiveDto { path, exported }))
}

#[tauri::command]
pub fn open_path(state: State<'_, DesktopState>, path: PathBuf) -> CommandResult<()> {
    let opener = TauriPathOpener::new(state.app_handle.clone());
    desktop_result(state.system.open_path(path, &opener))
}

#[tauri::command]
pub fn reveal_path(state: State<'_, DesktopState>, path: PathBuf) -> CommandResult<()> {
    let opener = TauriPathOpener::new(state.app_handle.clone());
    desktop_result(state.system.reveal_path(path, &opener))
}

#[tauri::command]
pub async fn bootstrap_app(state: State<'_, DesktopState>) -> CommandResult<AppBootstrapDto> {
    let bootstrap = state.host.bootstrap_app().await?;
    if let Some(workspace) = &bootstrap.workspace {
        desktop_result(state.system.allow_user_path(&workspace.root))?;
    }
    Ok(bootstrap)
}

#[tauri::command]
pub async fn open_workspace(
    state: State<'_, DesktopState>,
    request: OpenWorkspaceRequestDto,
) -> CommandResult<WorkspaceStatusDto> {
    let root = request.root.clone();
    state.abort_generation_worker_and_wait().await;
    let status = state.host.open_workspace(request).await?;
    desktop_result(state.system.allow_user_path(root))?;
    Ok(status)
}

#[tauri::command]
pub fn workspace_status(
    state: State<'_, DesktopState>,
) -> CommandResult<Option<WorkspaceStatusDto>> {
    state.host.workspace_status()
}

#[tauri::command]
pub async fn close_workspace(
    state: State<'_, DesktopState>,
) -> CommandResult<CloseWorkspaceResponseDto> {
    state.abort_generation_worker_and_wait().await;
    state.host.close_workspace()
}

fn image_resource_request_from_file(
    path: &Path,
    kind: ImageResourceKindDto,
) -> DesktopSystemResult<ImportImageResourceRequestDto> {
    Ok(ImportImageResourceRequestDto {
        kind,
        image_base64: STANDARD.encode(read_file_bytes(path)?),
        mime_type: None,
    })
}

fn vibe_document_request_from_file(
    path: &Path,
) -> DesktopSystemResult<ImportVibeDocumentRequestDto> {
    Ok(ImportVibeDocumentRequestDto {
        file_name: file_name(path),
        content: read_file_text(path)?,
    })
}

fn embedded_png_vibe_document_request_from_file(
    path: &Path,
) -> DesktopSystemResult<ImportEmbeddedPngVibeDocumentRequestDto> {
    Ok(ImportEmbeddedPngVibeDocumentRequestDto {
        file_name: file_name(path),
        png_bytes_base64: STANDARD.encode(read_file_bytes(path)?),
    })
}

fn read_file_bytes(path: &Path) -> DesktopSystemResult<Vec<u8>> {
    fs::read(path).map_err(|error| {
        DesktopSystemError::new(format!("failed to read file {}: {error}", path.display()))
    })
}

fn read_file_text(path: &Path) -> DesktopSystemResult<String> {
    fs::read_to_string(path).map_err(|error| {
        DesktopSystemError::new(format!(
            "failed to read text file {}: {error}",
            path.display()
        ))
    })
}

fn write_file_text(path: &Path, content: &str) -> DesktopSystemResult<()> {
    fs::write(path, content).map_err(|error| {
        DesktopSystemError::new(format!("failed to write file {}: {error}", path.display()))
    })
}

fn write_file_bytes(path: &Path, content: &[u8]) -> DesktopSystemResult<()> {
    fs::write(path, content).map_err(|error| {
        DesktopSystemError::new(format!("failed to write file {}: {error}", path.display()))
    })
}

fn image_extension(mime_type: Option<&str>) -> &'static str {
    match mime_type {
        Some("image/jpeg" | "image/jpg") => "jpg",
        Some("image/webp") => "webp",
        Some("image/gif") => "gif",
        _ => "png",
    }
}

fn resource_file_name(suggested_file_name: Option<String>, extension: &str) -> String {
    let Some(name) = suggested_file_name
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
    else {
        return format!("generation.{extension}");
    };
    if Path::new(&name).extension().is_some() {
        name
    } else {
        format!("{name}.{extension}")
    }
}

fn zip_file_name(suggested_file_name: Option<String>) -> String {
    let name = suggested_file_name
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "generation-batch".to_owned());
    if Path::new(&name).extension().is_some() {
        name
    } else {
        format!("{name}.zip")
    }
}

fn sanitize_archive_name(value: &str) -> String {
    let sanitized = value
        .trim()
        .chars()
        .map(|character| match character {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => character,
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "image".to_owned()
    } else {
        sanitized
    }
}

fn build_resource_images_zip(images: Vec<(String, Vec<u8>)>) -> CommandResult<Vec<u8>> {
    let cursor = Cursor::new(Vec::new());
    let mut archive = zip::ZipWriter::new(cursor);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (file_name, bytes) in images {
        archive
            .start_file(file_name, options)
            .map_err(|error| ErrorEnvelopeDto::new("zip_write_error", error.to_string()))?;
        archive
            .write_all(&bytes)
            .map_err(|error| ErrorEnvelopeDto::new("zip_write_error", error.to_string()))?;
    }
    archive
        .finish()
        .map(Cursor::into_inner)
        .map_err(|error| ErrorEnvelopeDto::new("zip_write_error", error.to_string()))
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|file_name| file_name.to_str())
        .unwrap_or("document")
        .to_owned()
}

fn desktop_result<T>(result: DesktopSystemResult<T>) -> CommandResult<T> {
    result.map_err(|error| ErrorEnvelopeDto::new("desktop_system_error", error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn image_resource_request_reads_selected_file_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("source.png");
        fs::write(&path, [0x89, b'P', b'N', b'G']).unwrap();

        let request =
            image_resource_request_from_file(&path, ImageResourceKindDto::SourceImage).unwrap();

        assert_eq!(request.kind, ImageResourceKindDto::SourceImage);
        assert_eq!(request.image_base64, "iVBORw==");
        assert_eq!(request.mime_type, None);
    }

    #[test]
    fn vibe_document_request_reads_selected_json_text() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("example.naiv4vibe");
        fs::write(&path, r#"{"name":"example"}"#).unwrap();

        let request = vibe_document_request_from_file(&path).unwrap();

        assert_eq!(request.file_name, "example.naiv4vibe");
        assert_eq!(request.content, r#"{"name":"example"}"#);
    }

    #[test]
    fn embedded_png_vibe_document_request_reads_selected_png_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("embedded.png");
        fs::write(&path, [1, 2, 3, 4]).unwrap();

        let request = embedded_png_vibe_document_request_from_file(&path).unwrap();

        assert_eq!(request.file_name, "embedded.png");
        assert_eq!(request.png_bytes_base64, "AQIDBA==");
    }

    #[test]
    fn resource_file_name_adds_detected_extension_when_missing() {
        assert_eq!(
            resource_file_name(Some("sample".to_owned()), "webp"),
            "sample.webp"
        );
        assert_eq!(
            resource_file_name(Some("sample.png".to_owned()), "webp"),
            "sample.png"
        );
        assert_eq!(image_extension(Some("image/jpeg")), "jpg");
    }

    #[test]
    fn resource_images_zip_preserves_stable_names_order_and_bytes() {
        let bytes = build_resource_images_zip(vec![
            ("request-01_sample-01.png".to_owned(), vec![1, 2, 3]),
            ("request-02_sample-01.webp".to_owned(), vec![4, 5]),
        ])
        .unwrap();
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        assert_eq!(archive.len(), 2);

        let mut first_bytes = Vec::new();
        let mut first = archive.by_index(0).unwrap();
        assert_eq!(first.name(), "request-01_sample-01.png");
        first.read_to_end(&mut first_bytes).unwrap();
        drop(first);

        let mut second_bytes = Vec::new();
        let mut second = archive.by_index(1).unwrap();
        assert_eq!(second.name(), "request-02_sample-01.webp");
        second.read_to_end(&mut second_bytes).unwrap();
        assert_eq!(first_bytes, vec![1, 2, 3]);
        assert_eq!(second_bytes, vec![4, 5]);
        assert_eq!(sanitize_archive_name("../bad:name"), ".._bad_name");
        assert_eq!(zip_file_name(Some("batch-1".to_owned())), "batch-1.zip");
    }

    #[test]
    fn file_read_errors_map_to_stable_desktop_error_code() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("missing.png");

        let error = desktop_result::<Vec<u8>>(read_file_bytes(&path)).unwrap_err();

        assert_eq!(error.code, "desktop_system_error");
        assert!(error.message.contains("failed to read file"));
    }
}
