/// WMTS response variants.
#[derive(Debug, Clone)]
pub enum WmtsResponse {
    /// XML response (GetCapabilities).
    Xml(String),
    /// Tile binary data (GetTile).
    Tile {
        /// Tile bytes.
        data: Vec<u8>,
        /// MIME type.
        mime_type: String,
    },
}
