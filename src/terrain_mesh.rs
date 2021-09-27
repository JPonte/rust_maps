use bevy::prelude::*;
use std::fs::File;

pub struct TerrainMeshOptions {
    pub width: u32,
    pub length: u32,
    pub height_scale: f32,
}

pub struct TerrainTile {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

fn get_normal(v1: &[f32; 3], v2: &[f32; 3], v3: &[f32; 3]) -> [f32; 3] {
    let a = [v2[0] - v1[0], v2[1] - v1[1], v2[2] - v1[2]];
    let b = [v3[0] - v1[0], v3[1] - v1[1], v3[2] - v1[2]];
    cross(&a, &b)
}

fn cross(a: &[f32; 3], b: &[f32; 3]) -> [f32; 3] {
    let nx = a[1] * b[2] - a[2] * b[1];
    let ny = a[2] * b[0] - a[0] * b[2];
    let nz = a[0] * b[1] - a[1] * b[0];
    [nx, ny, nz]
}

fn get_height(x: u32, y: u32, heightmap: &lerc::LercDataset) -> f32 {
    heightmap.data[x as usize + y as usize * heightmap.info.n_cols as usize] as f32
}

fn sample_heightmap(
    x: u32,
    y: u32,
    heightmap: &lerc::LercDataset,
    mesh_options: &TerrainMeshOptions,
) -> (f32, [f32; 3]) {
    let factor = (256. / (heightmap.data_range.z_max - heightmap.data_range.z_min)) as f32;

    let height = (get_height(x, y, heightmap) - heightmap.data_range.z_min as f32)
        * factor
        * mesh_options.height_scale;

    let target = [0., height, 0.];
    let right = [
        1.,
        if x >= (mesh_options.width - 1) {
            0.
        } else {
            get_height(x + 1, y, heightmap) * mesh_options.height_scale
        },
        0.,
    ];
    let left = [
        -1.,
        if x <= 1 {
            0.
        } else {
            get_height(x - 1, y, heightmap) * mesh_options.height_scale
        },
        0.,
    ];
    let top = [
        0.,
        if y >= (mesh_options.length - 1) {
            0.
        } else {
            get_height(x, y + 1, heightmap) * mesh_options.height_scale
        },
        1.,
    ];
    let bottom = [
        0.,
        if y <= 1 {
            0.
        } else {
            get_height(x, y - 1, heightmap) * mesh_options.height_scale
        },
        -1.,
    ];

    let normal_1 = get_normal(&target, &top, &right);
    let normal_2 = get_normal(&target, &bottom, &left);
    let new_normal = [
        (normal_1[0] + normal_2[0]) / 2.,
        (normal_1[1] + normal_2[1]) / 2.,
        (normal_1[2] + normal_2[2]) / 2.,
    ];

    (height, new_normal)
}

pub fn mesh_from_heightmap(
    filename: &str,
    mesh_options: TerrainMeshOptions,
    scale_factor: f32,
) -> Vec<([f32; 3], [f32; 3], [f32; 2])> {
    let lerc_dataset = lerc::decode_file(File::open(filename).unwrap());

    if let Ok(dataset) = lerc_dataset {
        println!(
            "Info: {:?} / Data Range: {:?} / Data length: {}",
            dataset.info,
            dataset.data_range,
            dataset.data.len()
        );

        let mut vertices_vec = Vec::new();
        for y in 0..(mesh_options.length - 0) {
            for x in 0..(mesh_options.width - 0) {
                let (height, normal) = sample_heightmap(x, y, &dataset, &mesh_options);
                let vertex = [
                    x as f32 * scale_factor,
                    height * scale_factor,
                    y as f32 * scale_factor,
                ];
                let uv = [
                    x as f32 / mesh_options.width as f32,
                    y as f32 / mesh_options.length as f32,
                ];
                vertices_vec.push((vertex, normal, uv));
            }
        }
        vertices_vec
    } else {
        println!("Failed to read {}", filename);
        Vec::new()
    }
}

const WIDTH: u32 = 257;
const LENGTH: u32 = 257;

pub fn setup_terrain(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    image_file: &str,
    topo_file: &str,
) {
    let texture_handle: Handle<Texture> = asset_server.load(image_file);
    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(texture_handle),
        roughness: 1.,
        metallic: 0.,
        reflectance: 0.,
        unlit: true,
        ..Default::default()
    });

    let scale_factor = 0.3;

    let vertices_vec = mesh_from_heightmap(
        topo_file,
        TerrainMeshOptions {
            width: LENGTH,
            length: WIDTH,
            height_scale: 0.1,
        },
        scale_factor,
    );

    let mut indices_vec = Vec::new();
    for y in 0..(LENGTH - 1) {
        for x in 0..(WIDTH - 1) {
            indices_vec.push(x + y * LENGTH);
            indices_vec.push(x + (y + 1) * LENGTH);
            indices_vec.push(x + 1 + y * LENGTH);

            indices_vec.push(x + 1 + y * LENGTH);
            indices_vec.push(x + (y + 1) * LENGTH);
            indices_vec.push(x + 1 + (y + 1) * LENGTH);
        }
    }

    let indices = bevy::render::mesh::Indices::U32(indices_vec);

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    for (position, normal, uv) in vertices_vec.iter() {
        positions.push(*position);
        normals.push(*normal);
        uvs.push(*uv);
    }

    let mut mesh = Mesh::new(bevy::render::pipeline::PrimitiveTopology::TriangleList);
    mesh.set_indices(Some(indices));
    mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    commands
        .spawn_bundle(PbrBundle {
            transform: Transform {
                translation: Vec3::new(
                    -(WIDTH as f32 * scale_factor / 2.),
                    0.,
                    -(LENGTH as f32 * scale_factor / 2.),
                ),
                ..Default::default()
            },
            mesh: meshes.add(mesh),
            material: material_handle,
            ..Default::default()
        })
        .insert(TerrainTile { x: 0, y: 0, z: 0 });
}
