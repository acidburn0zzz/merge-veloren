use super::super::{Mesh, Tri};
use zerocopy::AsBytes;

// gfx_defines! {
//     vertex Vertex {
//         pos: [f32; 2] = "v_pos",
//     }

//     constant Locals {
//         nul: [f32; 4] = "nul",
//     }

//     pipeline pipe {
//         vbuf: gfx::VertexBuffer<Vertex> = (),

//         locals: gfx::ConstantBuffer<Locals> = "u_locals",
//         globals: gfx::ConstantBuffer<Globals> = "u_globals",

//         src_sampler: gfx::TextureSampler<<WinColorFmt as
// gfx::format::Formatted>::View> = "src_color",

//         tgt_color: gfx::RenderTarget<WinColorFmt> = "tgt_color",
//         tgt_depth: gfx::DepthTarget<WinDepthFmt> =
// gfx::preset::depth::PASS_TEST,     }
// }

#[repr(C)]
#[derive(Copy, Clone, Debug, AsBytes)]
pub struct Vertex {
    pub pos: [f32; 2],
}

pub fn create_mesh() -> Mesh<Vertex> {
    let mut mesh = Mesh::new();

    #[rustfmt::skip]
    mesh.push_tri(Tri::new(
        Vertex { pos: [ 1.0, -1.0] },
        Vertex { pos: [-1.0,  1.0] },
        Vertex { pos: [-1.0, -1.0] },
    ));

    #[rustfmt::skip]
    mesh.push_tri(Tri::new(
        Vertex { pos: [1.0, -1.0] },
        Vertex { pos: [1.0,  1.0] },
        Vertex { pos: [-1.0, 1.0] },
    ));

    mesh
}
