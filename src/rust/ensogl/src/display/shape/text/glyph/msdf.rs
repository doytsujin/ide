//! Multichannel Signed Distance Field handling.

use crate::prelude::*;

use ensogl_core_msdf_sys as msdf_sys;
use msdf_sys::MultichannelSignedDistanceField;
use nalgebra::clamp;

// ====================
// === MSDF Texture ===
// ====================

/// Texture with msdf for all loaded glyph of font
///
/// This structure keeps texture data in 8-bit-per-channel RGB format, which
/// is ready to be passed to webgl texImage2D. The texture contains MSDFs for
/// all loaded glyph, organized in vertical column.
#[derive(Debug,Default)]
pub struct MsdfTexture {
    /// A plain data of this texture.
    data : RefCell<Vec<u8>>
}

impl MsdfTexture {
    /// Number of channels per cell in MSDF texture
    pub const CHANNELS_COUNT   : usize = MultichannelSignedDistanceField::CHANNELS_COUNT;
    /// Width of single MSDF in cells.
    pub const WIDTH            : usize = 32;
    /// Size of the texture row.
    pub const ROW_SIZE         : usize = Self::CHANNELS_COUNT * Self::WIDTH;
    /// Height of single MSDF in cells.
    pub const ONE_GLYPH_HEIGHT : usize = 32;
    /// Size of single MSDF.
    pub const ONE_GLYPH_SIZE   : usize = Self::ROW_SIZE * Self::ONE_GLYPH_HEIGHT;

    /// Number of rows in texture
    pub fn rows(&self) -> usize {
        self.data.borrow().len() / Self::ROW_SIZE
    }

    /// Do operation on borrowed texture data. Panics, if inside `operation` the texture data will
    /// be borrowed again (e.g. by calling `with_borrowed_data`.
    pub fn with_borrowed_data<F,R>(&self, operation:F) -> R
    where F : FnOnce(&Vec<u8>) -> R {
        let data = self.data.borrow();
        operation(&data)
    }

    /// Extends texture with new MSDF data in f32 format
    pub fn extend_f32<T:IntoIterator<Item=f32>>(&self, iter:T) {
        let f32_iterator       = iter.into_iter();
        let converted_iterator = f32_iterator.map(Self::convert_cell_from_f32);
        self.data.borrow_mut().extend(converted_iterator);
    }

    fn convert_cell_from_f32(value : f32) -> u8 {
        const UNSIGNED_BYTE_MIN : f32 = 0.0;
        const UNSIGNED_BYTE_MAX : f32 = 255.0;

        let scaled_to_byte  = value * UNSIGNED_BYTE_MAX;
        let clamped_to_byte = clamp(scaled_to_byte,UNSIGNED_BYTE_MIN,UNSIGNED_BYTE_MAX);
        clamped_to_byte as u8
    }
}



// ==================================
// === Msdf-sys Values Converting ===
// ==================================

/// Converts x dimension distance obtained from msdf-sys to vertex-space values
///
/// The values obtained from `msdf-sys` are expressed in MSDF cells. This
/// function convert them to normalized coordinates, where
/// (0.0, 0.0) is initial pen position for an character, and `y` = 1.0 is
/// _ascender_.
pub fn x_distance_from_msdf_value(msdf_value:f64) -> f32 {
    msdf_value as f32 / MsdfTexture::WIDTH as f32
}

/// Converts y dimension distance obtained from msdf-sys to vertex-space values
///
/// The values obtained from `msdf-sys` are expressed in MSDF cells. This
/// function convert them to normalized coordinates, where
/// (0.0, 0.0) is initial pen position for an character, and `y` = 1.0 is
/// _ascender_.
pub fn y_distance_from_msdf_value(msdf_value:f64) -> f32 {
    msdf_value as f32 / MsdfTexture::ONE_GLYPH_HEIGHT as f32
}

/// Converts translation obtained from msdf-sys to vertex-space values
///
/// This function get the transformation obtained from `msdf_sys` which is
/// expressed in MSDF units, and convert it to  normalized coordinates, where
/// (0.0, 0.0) is initial pen position for an character, and `y` = 1.0 is
/// _ascender_.
pub fn convert_msdf_translation(msdf:&MultichannelSignedDistanceField)
-> Vector2<f32> {
    let translate_converted_x = x_distance_from_msdf_value(msdf.translation.x);
    let translate_converted_y = y_distance_from_msdf_value(msdf.translation.y);
    Vector2::new(translate_converted_x, translate_converted_y)
}



#[cfg(test)]
mod test {
    use super::*;

    use wasm_bindgen_test::wasm_bindgen_test;

    #[test]
    fn extending_msdf_texture() {
        let texture = MsdfTexture::default();
        let msdf_values: &[f32] = &[-0.5, 0.0, 0.25, 0.5, 0.75, 1.0, 1.25];
        texture.extend_f32(msdf_values[..4].iter().cloned());
        texture.extend_f32(msdf_values[4..].iter().cloned());

        assert_eq!([0, 0, 63, 127, 191, 255, 255], texture.data.borrow().as_slice());
    }

    #[test]
    fn x_dimension_converting() {
        assert_eq!(1.0/8.0, x_distance_from_msdf_value(4.0));
        assert_eq!(1.0/2.0, x_distance_from_msdf_value(16.0));
    }

    #[test]
    fn y_dimension_converting() {
        assert_eq!(1.0/8.0, y_distance_from_msdf_value(4.0));
        assert_eq!(1.0/2.0, y_distance_from_msdf_value(16.0));
    }

    #[wasm_bindgen_test(async)]
    async fn msdf_translation_converting() {
        ensogl_core_msdf_sys::initialized().await;
        let mut msdf = MultichannelSignedDistanceField::mock_results();
        msdf.translation = Vector2::new(16.0, 4.0);

        let converted = convert_msdf_translation(&msdf);
        let expected = Vector2::new(0.5, 1.0/8.0);

        assert_eq!(expected, converted);
    }
}
