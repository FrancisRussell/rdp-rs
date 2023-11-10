/// Use to `to_utf16_le` function for String
pub trait Unicode {
    /// Convert any string into utf-16le string
    ///
    /// # Example
    /// ```
    /// use rdp::model::unicode::Unicode;
    /// let s = "foo".to_string();
    /// assert_eq!(s.to_utf16_le(), [102, 0, 111, 0, 111, 0])
    /// ```
    fn to_utf16_le(&self) -> Vec<u8>;
}

impl Unicode for &str {
    fn to_utf16_le(&self) -> Vec<u8> { self.encode_utf16().flat_map(u16::to_le_bytes).collect() }
}

impl Unicode for String {
    fn to_utf16_le(&self) -> Vec<u8> { self.as_str().to_utf16_le() }
}
