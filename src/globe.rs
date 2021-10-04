use bevy::{
    prelude::*,
    render::camera::*,
    tasks::{AsyncComputeTaskPool, Task},
};

use futures_lite::future;

mod camera_utils;
mod coord_utils;
use camera_utils::*;
mod map_services;
use map_services::*;

fn main() {
    App::build()
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(WindowDescriptor {
            title: "Globe".to_string(),
            ..Default::default()
        })
        .insert_resource(UserPosition::default())
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::rgb(0., 0., 0.)))
        .add_event::<MouseEvents>()
        .add_startup_system(setup.system())
        .add_system(emit_mouse_events.system())
        .add_system(new_camera_motion_system.system())
        .add_system(on_zoom_updated.system())
        .add_system(handle_tasks.system())
        .run();
}

const MAX_DIST: f32 = 20000.;
const GLOBE_RADIUS: f32 = 6000.;
const DIST_BUFFER: f32 = 2.;

struct GlobeTile;

#[derive(Default)]
struct UserPosition {
    tile_x: u32,
    tile_y: u32,
    zoom: u32,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut camera_transform = Transform::from_translation(Vec3::ZERO);
    camera_transform.look_at(Vec3::new(0., 0., 0.), Vec3::Y);

    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: camera_transform,
            perspective_projection: PerspectiveProjection {
                far: MAX_DIST + GLOBE_RADIUS,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(OrbitCamera {
            x: 180.,
            y: 30.,
            zoom: MAX_DIST,
        });

    commands.spawn_bundle(LightBundle {
        transform: Transform::from_translation(Vec3::new(100., 0., 0.)),
        light: Light {
            intensity: 15000.0,
            range: 200.0,
            ..Default::default()
        },
        ..Default::default()
    });

    asset_server.watch_for_changes().unwrap();
}

fn generate_tile(
    x: u32,
    y: u32,
    z: u32,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
) {
    let path = format!("images/imagery_{}_{}_{}.jpeg", 2_u32.pow(z) - x - 1, y, z);
    let texture_handle: Handle<Texture> = asset_server.load(&path[..]);

    let material = materials.add(StandardMaterial {
        roughness: 1.,
        metallic: 0.,
        // base_color: Color::rgb((x % 2) as f32, (y % 2) as f32, z as f32 / 13.),
        base_color_texture: Some(texture_handle),
        unlit: true,
        ..Default::default()
    });
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(tile_mesh(x, y, z)),
            material: material,
            ..Default::default()
        })
        .insert(GlobeTile);
}

fn tile_mesh(x: u32, y: u32, z: u32) -> Mesh {
    let n_vertices = 8;

    let n = 2_u32.pow(z);
    let theta = std::f32::consts::TAU / n as f32;
    let alpha = std::f32::consts::PI / n as f32;

    let theta_inc = theta / (n_vertices - 1) as f32;
    let alpha_inc = alpha / (n_vertices - 1) as f32;

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();

    for h in 0..n_vertices {
        let r = (alpha_inc * h as f32 + y as f32 * alpha).sin() * GLOBE_RADIUS;
        let vy = (alpha_inc * h as f32 + y as f32 * alpha).cos() * GLOBE_RADIUS;
        for w in 0..n_vertices {
            let vx = (theta_inc * w as f32 + x as f32 * theta).cos() * r;
            let vz = (theta_inc * w as f32 + x as f32 * theta).sin() * r;
            positions.push([vx, vy, vz]);
            normals.push([0., 0., 0.]);
            uvs.push([
                1. - w as f32 / (n_vertices - 1) as f32,
                h as f32 / (n_vertices - 1) as f32,
            ]);
        }
    }

    let mut indices_vec = Vec::new();
    for y in 0..(n_vertices - 1) {
        for x in 0..(n_vertices - 1) {
            indices_vec.push(x + y * n_vertices);
            indices_vec.push(x + 1 + y * n_vertices);
            indices_vec.push(x + (y + 1) * n_vertices);

            indices_vec.push(x + 1 + y * n_vertices);
            indices_vec.push(x + 1 + (y + 1) * n_vertices);
            indices_vec.push(x + (y + 1) * n_vertices);
        }
    }

    let indices = bevy::render::mesh::Indices::U32(indices_vec);

    let mut mesh = Mesh::new(bevy::render::pipeline::PrimitiveTopology::TriangleList);
    mesh.set_indices(Some(indices));
    mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    mesh
}

fn on_zoom_updated(
    query: Query<&OrbitCamera, Changed<OrbitCamera>>,
    current_tiles: Query<Entity, With<GlobeTile>>,
    mut res: ResMut<UserPosition>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    thread_pool: Res<AsyncComputeTaskPool>,
) {
    if let Ok(camera) = query.single() {
        let camera_dist = camera.zoom;

        let x = magic_eq(GLOBE_RADIUS, camera_dist - GLOBE_RADIUS - DIST_BUFFER) * 2.;
        let new_zoom = (2. * std::f32::consts::PI / x)
            .log2()
            .clamp(1., 13.)
            .round() as u32;

        let cam_x = (360. - camera.x).rem_euclid(360.);
        let cam_y = if camera.y >= 270. {
            180. - (camera.y % 270.)
        } else {
            90. - camera.y
        };
        let new_tile_x = (cam_x / (360. / 2_i32.pow(new_zoom) as f32)).floor() as u32;
        let new_tile_y = (cam_y / (180. / 2_i32.pow(new_zoom) as f32)).floor() as u32;

        if new_zoom != res.zoom || new_tile_x != res.tile_x || new_tile_y != res.tile_y {
            res.tile_x = new_tile_x;
            res.tile_y = new_tile_y;
            res.zoom = new_zoom;

            for tile in current_tiles.iter() {
                commands.entity(tile).despawn();
            }

            let radius_x = 2;
            let radius_y = 2;

            for r_x in -radius_x..(radius_x + 1) {
                for r_y in -radius_y..(radius_y + 1) {
                    let x = (new_tile_x as i32 + r_x).rem_euclid(2_i32.pow(new_zoom));
                    let y = (new_tile_y as i32 + r_y).rem_euclid(2_i32.pow(new_zoom));

                    request_tile(x as u32, y as u32, new_zoom, &mut commands, &thread_pool);

                    generate_tile(
                        x as u32,
                        y as u32,
                        new_zoom,
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        &asset_server,
                    );
                }
            }
        }
    }
}

pub fn new_camera_motion_system(
    time: Res<Time>,
    mut events: EventReader<MouseEvents>,
    mut query: Query<(&mut Transform, &mut OrbitCamera)>,
) {
    if let Ok((mut camera_transform, mut camera)) = query.single_mut() {
        for event in events.iter() {
            match event {
                &MouseEvents::Drag(delta) => {
                    let drag_speed =
                        20. * ((camera.zoom - GLOBE_RADIUS) / (MAX_DIST - GLOBE_RADIUS)).powi(1);
                    camera.x -= delta.x * time.delta_seconds() * drag_speed;
                    camera.y += delta.y * time.delta_seconds() * drag_speed;

                    while camera.x > 360. {
                        camera.x -= 360.;
                    }
                    while camera.x < 0. {
                        camera.x += 360.;
                    }

                    while camera.y > 360. {
                        camera.y -= 360.;
                    }
                    while camera.y < 0. {
                        camera.y += 360.;
                    }
                }
                &MouseEvents::Zoom(delta) => {
                    camera.zoom = (camera.zoom - delta * time.delta_seconds() * 400.)
                        .clamp(GLOBE_RADIUS + DIST_BUFFER, MAX_DIST);
                }
            }
        }
        camera_transform.translation = Quat::from_axis_angle(Vec3::Y, camera.x.to_radians())
            * Quat::from_axis_angle(Vec3::Z, camera.y.to_radians())
            * (Vec3::X * camera.zoom);
        camera_transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}

fn magic_eq(radius: f32, dist: f32) -> f32 {
    (1. / 8.) * (8. * ((2. - 2_f32.sqrt()).sqrt() * dist / (2. * radius)).asin())
}

fn request_tile(
    x: u32,
    y: u32,
    z: u32,
    commands: &mut Commands,
    thread_pool: &Res<AsyncComputeTaskPool>,
) {
    let task = thread_pool.spawn(async move {
        async_compat::Compat::new(async {
            match get_arcgis_image_tile(2_u32.pow(z) - x - 1, y, z).await {
                Ok(_) => {}
                Err(error) => {
                    println!("Failed to download tile ({}, {}, {}): {:?}", x, y, z, error);
                }
            }
            ()
        })
        .await;
    });
    commands.spawn().insert(task);
}

fn handle_tasks(mut commands: Commands, mut query_tasks: Query<(Entity, &mut Task<()>)>) {
    for (entity, mut task) in query_tasks.iter_mut() {
        if let Some(_) = future::block_on(future::poll_once(&mut *task)) {
            commands.entity(entity).remove::<Task<()>>().despawn();
        }
    }
}
