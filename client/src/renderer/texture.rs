use image::io::Reader as ImageReader;

#[derive(Debug)]
pub enum ImageError {
    Io(std::io::Error),
    Decode(String),
    Unsupported(String),
    OpenGL()
}

impl std::fmt::Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => f.write_str(format!("ImageError: IO error: {err}").as_str()),
            Self::Decode(err) => f.write_str(format!("ImageError: Decode error: {err}").as_str()),
            Self::Unsupported(err) => f.write_str(format!("ImageError: Unsupported error: {err}").as_str()),
            Self::OpenGL() => f.write_str("ImageError: OpenGL error")
        }
    }
}

impl From<std::io::Error> for ImageError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<image::ImageError> for ImageError {
    fn from(value: image::ImageError) -> Self {
        Self::Decode(value.to_string())
    }
}

pub struct Texture {
    id: u32,
    width: u32,
    height: u32,
    internal_format: gl::types::GLenum,
    data_format: gl::types::GLenum
}

impl Texture {
    pub(super) fn load_from(img_path: &str) -> Result<Self, ImageError> {
        let img = ImageReader::open(img_path)?
            .decode()?
            .flipv();
        
        let mut t = Self {
            id: 0,
            width: img.width(),
            height: img.height(),
            internal_format: 0,
            data_format: 0
        };

        match img {
            image::DynamicImage::ImageRgb8(_) => {
                t.internal_format = gl::RGB8;
                t.data_format = gl::RGB;
            }
            image::DynamicImage::ImageRgba8(_) => {
                t.internal_format = gl::RGBA8;
                t.data_format = gl::RGBA;
            }
            _ => return Err(ImageError::Unsupported(format!("Image format {img:?} is unsupported")))
        }

        unsafe {
            gl::CreateTextures(gl::TEXTURE_2D, 1, &mut t.id);
            if t.id == 0 {
                return Err(ImageError::OpenGL());
            }
            gl::TextureStorage2D(t.id, 1, t.internal_format, t.width as i32, t.height as i32);

            gl::TextureParameteri(t.id, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TextureParameteri(t.id, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);

            gl::TextureParameteri(t.id, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
            gl::TextureParameteri(t.id, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);

            gl::TextureSubImage2D(t.id, 0, 0, 0, t.width as i32, t.height as i32, t.data_format, gl::UNSIGNED_BYTE, img.into_bytes().as_ptr() as *const _)
        }

        Ok(t)
    }

    pub(super) fn create(width: u32, height: u32) -> Result<Self, ImageError> {
        let mut t = Self {
            id: 0,
            width,
            height,
            internal_format: gl::RGBA8,
            data_format: gl::RGBA
        };

        unsafe {
            gl::CreateTextures(gl::TEXTURE_2D, 1, &mut t.id);
            if t.id == 0 {
                return Err(ImageError::OpenGL());
            }
            gl::TextureStorage2D(t.id, 1, t.internal_format, t.width as i32, t.height as i32);

            gl::TextureParameteri(t.id, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TextureParameteri(t.id, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);

            gl::TextureParameteri(t.id, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
            gl::TextureParameteri(t.id, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
        }

        Ok(t)
    }

    pub(super) fn set_data(&self, data: &[u8]) {
        let bytes_per_pixel = match self.data_format {
            gl::RGBA => 4,
            gl::RGB => 3,
            _ => panic!("unsupported texture data format {}", self.data_format)
        };

        assert_eq!(data.len() as u32, self.width * self.height * bytes_per_pixel, "wrong pixel data size for texture");
        unsafe {
            gl::TextureSubImage2D(
                self.id, 
                0, 
                0, 0, 
                self.width as i32, self.height as i32,
                self.data_format, gl::UNSIGNED_BYTE, 
                data.as_ptr() as *const _
            );
        }
    }

    pub(super) fn bind(&self, slot: u32) {
        unsafe { gl::BindTextureUnit(slot, self.id); }
    }
}