use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BufferType {
    Array = gl::ARRAY_BUFFER as isize,
    ElementArray = gl::ELEMENT_ARRAY_BUFFER as isize
}

pub(super) struct Buffer {
    id: RenderID,
    typ: BufferType,
    layout: Option<BufferLayout>,
    count: u32
}

impl Buffer {
    pub(super) fn new(typ: BufferType, data: &[u8], layout: Option<BufferLayout>) -> Option<Self> {
        let mut id = 0;
        unsafe { 
            gl::CreateBuffers(1, &mut id);
            gl::BindBuffer(typ as gl::types::GLenum, id);
            gl::BufferData(typ as gl::types::GLenum, data.len() as isize, data.as_ptr() as *const _, gl::STATIC_DRAW);
        }
        if id != 0 {
            Some(Self {
                id,
                typ,
                layout,
                count: (data.len() / std::mem::size_of::<gl::types::GLuint>()) as u32
            })
        }
        else {
            None
        }
    }

    pub(super) fn delete(self) {
        unsafe { gl::DeleteBuffers(1, &self.id) }
    }

    pub(super) fn bind(&self) {
        unsafe { gl::BindBuffer(self.typ as gl::types::GLenum, self.id) }
    }

    pub(super) fn unbind(&self) {
        unsafe { gl::BindBuffer(self.typ as gl::types::GLenum, 0) }
    } 

    pub(super) fn layout(&self) -> &Option<BufferLayout> {
        &self.layout
    }

    pub(super) fn count(&self) -> u32 {
        self.count
    }
}

pub(super) struct BufferElement {
    typ: ShaderDataType,
    offset: u32,
    normalized: bool
}

impl BufferElement {
    pub(super) fn new(typ: ShaderDataType) -> Self {
        Self {
            typ,
            offset: 0,
            normalized: false
        }
    }

    pub(super) fn size(&self) -> u32 {
        self.typ.size()
    }

    pub(super) fn component_count(&self) -> i32 {
        self.typ.component_count()
    }

    pub(super) fn base_type(&self) -> gl::types::GLenum {
        self.typ.base_type()
    }

    pub(super) fn offset(&self) -> u32 {
        self.offset
    }

    pub(super) fn set_offset(&mut self, offset: u32) {
        self.offset = offset
    }

    pub(super) fn normalized(&self) -> gl::types::GLboolean {
        if self.normalized {
            gl::TRUE
        }
        else {
            gl::FALSE
        }
    }
}

pub(super) struct BufferLayout {
    elements: Vec<BufferElement>,
    stride: u32
}

impl BufferLayout {
    pub(super) fn new(elements: Vec<BufferElement>) -> Self {
        let mut buffer = Self {
            elements,
            stride: 0
        };
        buffer.calculate_offsets_and_stride();
        buffer
    }
    
    pub(super) fn stride(&self) -> u32 {
        self.stride
    }

    pub(super) fn elements(&self) -> &Vec<BufferElement> {
        &self.elements
    }

    fn calculate_offsets_and_stride(&mut self) {
        let mut offset = 0;
        self.stride = 0;
        for element in self.elements.iter_mut() {
            element.set_offset(offset);
            offset += element.size();
            self.stride += element.size();
        }
    }
}