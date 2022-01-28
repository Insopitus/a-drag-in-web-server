use std::{
    fs::{self, read_dir, File},
    io::{self, BufReader, Read},
    path::Path,
};
const CHUNK_SIZE: usize = 1024*64; // 64kb

/// A class to reader file from the given path
///
/// it's instanized once and use by all the threads, be careful.
pub struct FolderReader {
    root_path: String,
}
impl FolderReader {
    /// `path`: the root path of the reader, all request path are relative to this path.
    ///
    /// `chunk_size`: how many bytes can be read at once (to limit memory usage)
    pub fn new(path: &Path) -> FolderReader {
        let metadata = fs::metadata(path).expect("Failed to read the directory"); //should throw an error showing `failed to read the directory`
        let mut root_path;
        let path = path.to_str().expect("Invalid path."); //should throw an error showing `invalid path`
        if metadata.is_dir() {
            root_path = path.to_string();
        } else {
            let mut a:Vec<&str> = path.split("\\").collect();
            a.pop();

            root_path = a.join("\\");
        }
        if !root_path.ends_with("\\") {
            root_path.push_str("\\");
        }
        FolderReader { root_path }
    }
    pub fn root_path(&self) -> &str {
        &self.root_path
    }
    ///
    fn get_full_path_from_relative(&self, dir: &str) -> String {
        let mut file_path = self.root_path.clone();
        file_path.push_str(dir);

        file_path
    }
    pub fn get_file_size(&self, dir: &str) -> Result<u64, io::Error> {
        let file_path = self.get_full_path_from_relative(dir);
        Ok(fs::metadata(file_path)?.len())
    }
    pub fn get_file_as_string(&self, dir: &str) -> Result<String, io::Error> {
        let file_path = self.get_full_path_from_relative(dir);
        fs::read_to_string(file_path)
    }
    pub fn get_file_as_bytes(&self, dir: &str) -> Result<Vec<u8>, io::Error> {
        let file_path = self.get_full_path_from_relative(dir);
        fs::read(file_path)
    }
    pub fn get_chunked_file_as_bytes(&self, dir: &str) -> Result<FileChunksReader, io::Error> {
        let file_path = self.get_full_path_from_relative(dir);
        let metadata = fs::metadata(&file_path)?;
        let length = metadata.len();
        let file = File::open(file_path)?;
        let chunks = FileChunksReader {
            file: BufReader::new(file),
            content: [0u8; CHUNK_SIZE],
            bytes_remaining: length.try_into().unwrap_or(0),
        };
        Ok(chunks)
    }
    /// recursively enumerate all the files in the path
    fn _visit_dir(&self, path: &Path, info: &mut String) -> Result<(), std::io::Error> {
        for entry in read_dir(path)? {
            let entry = entry?;
            let dir = entry.path();
            if dir.is_dir() {
                self._visit_dir(&dir, info)?;
            } else if let Some(str) = dir.to_str() {
                info.push_str(str);
                info.push_str("\n")
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct FileChunksReader {
    file: BufReader<File>,
    content: [u8; CHUNK_SIZE],
    bytes_remaining: i64,
}
impl Iterator for FileChunksReader {
    type Item = [u8; CHUNK_SIZE];
    fn next(&mut self) -> Option<Self::Item> {
        if self.bytes_remaining > 0 {
            match self.file.read(&mut self.content) {
                Ok(_) => {
                    self.bytes_remaining -= CHUNK_SIZE.try_into().unwrap_or(1024*1024);
                    Some(self.content)
                }
                Err(e) => match e.kind() {
                    io::ErrorKind::UnexpectedEof => None,
                    _ => None,
                },
            }
        } else {
            None
        }
    }
}
