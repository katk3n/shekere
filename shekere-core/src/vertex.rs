#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
pub struct Vertex {
    position: [f32; 3],
}

impl Vertex {
    #[cfg(test)]
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: [x, y, z],
        }
    }

    #[cfg(test)]
    pub fn position(&self) -> [f32; 3] {
        self.position
    }

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            }],
        }
    }
}

pub const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-1.0, 1.0, 0.0],
    },
    Vertex {
        position: [-1.0, -1.0, 0.0],
    },
    Vertex {
        position: [1.0, -1.0, 0.0],
    },
    Vertex {
        position: [1.0, 1.0, 0.0],
    },
];

pub const INDICES: &[u16] = &[0, 1, 2, 3, 0, 2];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_new() {
        let vertex = Vertex::new(1.0, 2.0, 3.0);
        assert_eq!(vertex.position, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_vertex_position() {
        let vertex = Vertex::new(-0.5, 0.5, 0.0);
        assert_eq!(vertex.position(), [-0.5, 0.5, 0.0]);
    }

    #[test]
    fn test_vertex_desc() {
        let desc = Vertex::desc();
        assert_eq!(desc.array_stride, 12);
        assert_eq!(desc.step_mode, wgpu::VertexStepMode::Vertex);
        assert_eq!(desc.attributes.len(), 1);
        assert_eq!(desc.attributes[0].offset, 0);
        assert_eq!(desc.attributes[0].shader_location, 0);
        assert_eq!(desc.attributes[0].format, wgpu::VertexFormat::Float32x3);
    }

    #[test]
    fn test_vertex_bytemuck() {
        let vertex = Vertex::new(1.5, -2.5, 3.5);
        let vertex_array = [vertex];
        let bytes: &[u8] = bytemuck::cast_slice(&vertex_array);
        assert_eq!(bytes.len(), 12);

        let reconstructed: Vertex = bytemuck::cast_slice::<u8, Vertex>(bytes)[0];
        assert_eq!(reconstructed.position, [1.5, -2.5, 3.5]);
    }

    #[test]
    fn test_vertex_equality() {
        let vertex1 = Vertex::new(1.0, 2.0, 3.0);
        let vertex2 = Vertex::new(1.0, 2.0, 3.0);
        let vertex3 = Vertex::new(1.0, 2.0, 4.0);

        assert_eq!(vertex1, vertex2);
        assert_ne!(vertex1, vertex3);
    }

    #[test]
    fn test_vertices_constant() {
        assert_eq!(VERTICES.len(), 4);
        assert_eq!(VERTICES[0].position, [-1.0, 1.0, 0.0]);
        assert_eq!(VERTICES[1].position, [-1.0, -1.0, 0.0]);
        assert_eq!(VERTICES[2].position, [1.0, -1.0, 0.0]);
        assert_eq!(VERTICES[3].position, [1.0, 1.0, 0.0]);
    }

    #[test]
    fn test_indices_constant() {
        assert_eq!(INDICES.len(), 6);
        assert_eq!(INDICES, &[0, 1, 2, 3, 0, 2]);
    }
}
