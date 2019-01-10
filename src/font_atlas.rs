use std::fs::File;
use std::path::Path;
use std::collections::HashMap;


#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Address {
    pub row: usize,
    pub column: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FontAtlas {
    pub glyph_y_offsets: HashMap<char, f32>,
    pub glyph_widths: HashMap<char, f32>,
    pub glyph_coords: HashMap<char, Address>,
    pub rows: usize,
    pub columns: usize,
}

#[derive(Debug, Clone)]
pub enum Error {
    FileNotFound(String),
    CouldNotParseFontFile(String),
}

pub fn load<P: AsRef<Path>>(file: P) -> Result<FontAtlas, Error> {
    let data = match File::open(file.as_ref()) {
        Ok(handle) => handle,
        Err(_) => {
            return Err(
                Error::FileNotFound(format!("{}", file.as_ref().display()))
            );
        }
    };
    let font_atlas = match serde_json::from_reader(data) {
        Ok(val) => val,
        Err(_) => {
            return Err(
                Error::CouldNotParseFontFile(format!("{}", file.as_ref().display()))
            );
        }
    };

    Ok(font_atlas)
}
