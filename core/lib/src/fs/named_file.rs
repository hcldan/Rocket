use std::io;
use std::path::{Path, PathBuf};
use std::ops::{Deref, DerefMut};

use tokio::fs::File;

use crate::request::Request;
use crate::response::{self, Responder};
use crate::http::ContentType;

/// A [`Responder`] that sends file data with a Content-Type based on its
/// file extension.
///
/// # Example
///
/// A simple static file server mimicking [`FileServer`]:
///
/// ```rust
/// # use rocket::get;
/// use std::path::{PathBuf, Path};
///
/// use rocket::fs::{NamedFile, relative};
///
/// #[get("/file/<path..>")]
/// pub async fn second(path: PathBuf) -> Option<NamedFile> {
///     let mut path = Path::new(relative!("static")).join(path);
///     if path.is_dir() {
///         path.push("index.html");
///     }
///
///     NamedFile::open(path).await.ok()
/// }
/// ```
///
/// Always prefer to use [`FileServer`] which has more functionality and a
/// pithier API.
///
/// [`FileServer`]: crate::fs::FileServer
#[derive(Debug)]
pub struct NamedFile {
    path: PathBuf, 
    file: File,
    /// If file ends in .gz, set `Content-Encoding` to gzip and use the base 
    /// extension for `Content-Type`
    compressed: bool,
}

impl NamedFile {
    /// Attempts to open a file in read-only mode.
    ///
    /// # Errors
    ///
    /// This function will return an error if path does not already exist. Other
    /// errors may also be returned according to
    /// [`OpenOptions::open()`](std::fs::OpenOptions::open()).
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::get;
    /// use rocket::fs::NamedFile;
    ///
    /// #[get("/")]
    /// async fn index() -> Option<NamedFile> {
    ///     NamedFile::open("index.html").await.ok()
    /// }
    /// ```
    pub async fn open<P: AsRef<Path>>(path: P) -> io::Result<NamedFile> {
        // FIXME: Grab the file size here and prohibit `seek`ing later (or else
        // the file's effective size may change), to save on the cost of doing
        // all of those `seek`s to determine the file size. But, what happens if
        // the file gets changed between now and then?
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path).await?;
        Ok(NamedFile { path, file, compressed: false })
    }

    pub async fn open_compressed<P: AsRef<Path>>(path: P) -> io::Result<NamedFile> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path).await?;
        Ok(NamedFile { path, file, compressed: true })
    }

    /// Retrieve the underlying `File`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fs::NamedFile;
    ///
    /// # async fn f() -> std::io::Result<()> {
    /// let named_file = NamedFile::open("index.html").await?;
    /// let file = named_file.file();
    /// # Ok(())
    /// # }
    /// ```
    #[inline(always)]
    pub fn file(&self) -> &File {
        &self.file
    }

    /// Retrieve a mutable borrow to the underlying `File`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fs::NamedFile;
    ///
    /// # async fn f() -> std::io::Result<()> {
    /// let mut named_file = NamedFile::open("index.html").await?;
    /// let file = named_file.file_mut();
    /// # Ok(())
    /// # }
    /// ```
    #[inline(always)]
    pub fn file_mut(&mut self) -> &mut File {
        &mut self.file
    }

    /// Take the underlying `File`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fs::NamedFile;
    ///
    /// # async fn f() -> std::io::Result<()> {
    /// let named_file = NamedFile::open("index.html").await?;
    /// let file = named_file.take_file();
    /// # Ok(())
    /// # }
    /// ```
    #[inline(always)]
    pub fn take_file(self) -> File {
        self.file
    }

    /// Retrieve the path of this file.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rocket::fs::NamedFile;
    ///
    /// # async fn demo_path() -> std::io::Result<()> {
    /// let file = NamedFile::open("foo.txt").await?;
    /// assert_eq!(file.path().as_os_str(), "foo.txt");
    /// # Ok(())
    /// # }
    /// ```
    #[inline(always)]
    pub fn path(&self) -> &Path {
        &self.path.as_path()
    }
}

/// Streams the named file to the client. Sets or overrides the Content-Type in
/// the response according to the file's extension if the extension is
/// recognized. See [`ContentType::from_extension()`] for more information. If
/// you would like to stream a file with a different Content-Type than that
/// implied by its extension, use a [`File`] directly.
impl<'r> Responder<'r, 'static> for NamedFile {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        let mut response = self.file.respond_to(req)?;
        if let Some(mut ext) = self.path.extension() {
            let stripped = self.path.with_extension("");

            if self.compressed && ext == std::ffi::OsStr::new("gz") {
                response.set_raw_header("Content-Encoding", "gzip");
                if let Some(orig_ext) = stripped.extension() {
                    ext = orig_ext; // override extension-based content type
                }
            }
            if let Some(ct) = ContentType::from_extension(&ext.to_string_lossy()) {
                response.set_header(ct);
            }
        }

        Ok(response)
    }
}

impl Deref for NamedFile {
    type Target = File;

    fn deref(&self) -> &File {
        &self.file
    }
}

impl DerefMut for NamedFile {
    fn deref_mut(&mut self) -> &mut File {
        &mut self.file
    }
}
