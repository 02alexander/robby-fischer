use rerun::{Color, Mesh3D, Rgba32};

// /// Assumes the gltf consists of a single mesh with identity transform.
// pub fn load_gltf_as_mesh3d(path: impl AsRef<Path>) -> anyhow::Result<rerun::Mesh3D> {
//     let (doc, buffers, _) = gltf::import_slice(bytes::Bytes::from(std::fs::read(path)?))?;
//     let nodes = load_gltf(&doc, &buffers);

//     // let mut children = nodes.into_iter().next().unwrap().children;
//     // let mut primitives = children.remove(0).primitives;
//     // let primitive = primitives.remove(0);
//     // Ok(rerun::Mesh3D::from(primitive))

//     let rec = rerun::RecordingStream::thread_local(rerun::StoreKind::Recording).unwrap();
//     for node in nodes {
//         println!("nodes");
//         dbg!(node.primitives.len());
//         dbg!(node.children.len());
//         log_node(&rec, node)?;
//     }
//     Err(anyhow!("error"))

// }

// Declare how to turn a glTF primitive into a Rerun component (`Mesh3D`).
#[allow(clippy::fallible_impl_from)]
impl From<GltfPrimitive> for Mesh3D {
    fn from(primitive: GltfPrimitive) -> Self {
        let GltfPrimitive {
            albedo_factor,
            indices,
            vertex_positions,
            vertex_colors,
            vertex_normals,
            vertex_texcoords,
        } = primitive;

        let mut mesh = Mesh3D::new(vertex_positions);

        if let Some(indices) = indices {
            assert!(indices.len() % 3 == 0);
            let triangle_indices = indices.chunks_exact(3).map(|tri| (tri[0], tri[1], tri[2]));
            mesh = mesh.with_triangle_indices(triangle_indices);
        }
        if let Some(vertex_normals) = vertex_normals {
            mesh = mesh.with_vertex_normals(vertex_normals);
        }
        if let Some(vertex_colors) = vertex_colors {
            mesh = mesh.with_vertex_colors(vertex_colors);
        }
        if let Some(vertex_texcoords) = vertex_texcoords {
            mesh = mesh.with_vertex_texcoords(vertex_texcoords);
        }
        if albedo_factor.is_some() {
            mesh = mesh.with_mesh_material(rerun::datatypes::Material {
                albedo_factor: albedo_factor.map(|[r, g, b, a]| {
                    Rgba32::from_unmultiplied_rgba(
                        (r * 255.0).clamp(0.0, 255.0) as u8,
                        (g * 255.0).clamp(0.0, 255.0) as u8,
                        (b * 255.0).clamp(0.0, 255.0) as u8,
                        (a * 255.0).clamp(0.0, 255.0) as u8,
                    )
                }),
            });
        }

        mesh.sanity_check().unwrap();

        mesh
    }
}

// Declare how to turn a glTF transform into a Rerun component (`Transform`).
impl From<GltfTransform> for rerun::Transform3D {
    fn from(transform: GltfTransform) -> Self {
        rerun::Transform3D::from_translation_rotation_scale(
            transform.t,
            rerun::datatypes::Quaternion::from_xyzw(transform.r),
            transform.s,
        )
    }
}

/// Log a glTF node with Rerun.
pub fn log_node(
    rec: &rerun::RecordingStream,
    base_path: &str,
    node: GltfNode,
) -> anyhow::Result<()> {
    if let Some(transform) = node.transform.map(rerun::Transform3D::from) {
        rec.log(format!("{}/{}", base_path, node.name.as_str()), &transform)?;
    }

    // Convert glTF objects into Rerun components.
    for (i, primitive) in node.primitives.into_iter().enumerate() {
        let mesh: rerun::Mesh3D = primitive.into();
        rec.log(format!("{}/{}/{}", base_path, node.name, i), &mesh)?;
    }

    // Recurse through all of the node's children!
    for mut child in node.children {
        child.name = [node.name.as_str(), child.name.as_str()].join("/");
        log_node(rec, base_path, child)?;
    }

    Ok(())
}

// fn run(rec: &rerun::RecordingStream) -> anyhow::Result<()> {
//     // Read glTF scene
//     let (doc, buffers, _) = gltf::import_slice(bytes::Bytes::from(std::fs::read(args.scene_path()?)?))?;
//     let nodes = load_gltf(&doc, &buffers);

//     // Log raw glTF nodes and their transforms with Rerun
//     for root in nodes {
//         rec.log_static(root.name.as_str(), &rerun::ViewCoordinates::RIGHT_HAND_Y_UP)?;
//         log_node(rec, root)?;
//     }

//     Ok(())
// }

#[derive(Clone)]
pub struct GltfNode {
    name: String,
    transform: Option<GltfTransform>,
    primitives: Vec<GltfPrimitive>,
    children: Vec<GltfNode>,
}

#[derive(Clone)]
pub struct GltfPrimitive {
    albedo_factor: Option<[f32; 4]>,
    indices: Option<Vec<u32>>,
    vertex_positions: Vec<[f32; 3]>,
    vertex_colors: Option<Vec<Color>>,
    vertex_normals: Option<Vec<[f32; 3]>>,
    #[allow(dead_code)]
    vertex_texcoords: Option<Vec<[f32; 2]>>,
}

#[derive(Clone)]
pub struct GltfTransform {
    t: [f32; 3],
    r: [f32; 4],
    #[allow(dead_code)]
    s: [f32; 3],
}

impl GltfNode {
    pub fn from_gltf(buffers: &[gltf::buffer::Data], node: &gltf::Node<'_>) -> Self {
        let name = node_name(node);

        let transform = {
            let (t, r, s) = node.transform().decomposed();
            GltfTransform { t, r, s }
        };
        let primitives = node_primitives(buffers, node).collect();

        let children = node
            .children()
            .map(|child| GltfNode::from_gltf(buffers, &child))
            .collect();

        Self {
            name,
            transform: Some(transform),
            primitives,
            children,
        }
    }
}

pub fn node_name(node: &gltf::Node<'_>) -> String {
    node.name()
        .map_or_else(|| format!("node_{}", node.index()), ToOwned::to_owned)
}

pub fn node_primitives<'data>(
    buffers: &'data [gltf::buffer::Data],
    node: &'data gltf::Node<'_>,
) -> impl Iterator<Item = GltfPrimitive> + 'data {
    node.mesh().into_iter().flat_map(|mesh| {
        mesh.primitives().map(|primitive| {
            assert!(primitive.mode() == gltf::mesh::Mode::Triangles);

            let albedo_factor = primitive
                .material()
                .pbr_metallic_roughness()
                .base_color_factor()
                .into();

            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            let indices = reader.read_indices();
            let indices = indices.map(|indices| indices.into_u32().collect());

            let vertex_positions = reader.read_positions().unwrap();
            let vertex_positions = vertex_positions.collect();

            let vertex_normals = reader.read_normals();
            let vertex_normals = vertex_normals.map(|normals| normals.collect());

            let vertex_colors = reader.read_colors(0);
            let vertex_colors = vertex_colors.map(|colors| {
                colors
                    .into_rgba_u8()
                    .map(|[r, g, b, a]| Color::from_unmultiplied_rgba(r, g, b, a))
                    .collect()
            });

            let vertex_texcoords = reader.read_tex_coords(0);
            let vertex_texcoords = vertex_texcoords.map(|texcoords| texcoords.into_f32().collect());

            GltfPrimitive {
                albedo_factor,
                vertex_positions,
                indices,
                vertex_normals,
                vertex_colors,
                vertex_texcoords,
            }
        })
    })
}

pub fn load_gltf<'data>(
    doc: &'data gltf::Document,
    buffers: &'data [gltf::buffer::Data],
) -> impl Iterator<Item = GltfNode> + 'data {
    doc.scenes().map(move |scene| {
        let name = scene
            .name()
            .map_or_else(|| format!("scene_{}", scene.index()), ToOwned::to_owned);

        GltfNode {
            name,
            transform: None,
            primitives: Default::default(),
            children: scene
                .nodes()
                .map(|node| GltfNode::from_gltf(buffers, &node))
                .collect(),
        }
    })
}
