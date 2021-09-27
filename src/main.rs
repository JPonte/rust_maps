use bevy::{
    prelude::*,
    render::camera::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use futures_lite::future;

mod terrain_mesh;
use terrain_mesh::*;

mod coord_utils;
use coord_utils::*;

mod camera_utils;
use camera_utils::*;

mod map_services;
use map_services::*;

struct UserPosition {
    lat: f64,
    lon: f64,
    zoom: u32,
}

fn main() {
    App::build()
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(WindowDescriptor {
            title: "Rust Maps".to_string(),
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::rgb(0.3, 0.56, 0.83)))
        .add_event::<MouseEvents>()
        .add_startup_system(query_stuff.system())
        .add_startup_system(setup.system())
        .add_system(emit_mouse_events.system())
        .add_system(camera_motion_system.system())
        .add_system(handle_tasks.system())
        .run();
}

fn query_stuff(mut commands: Commands, thread_pool: Res<AsyncComputeTaskPool>) {
    let task = thread_pool.spawn(async move {
        let lat = 36.253128;
        let lon = -112.521346;
        let z = 13;

        let (x, y) = deg2num(lat, lon, z);

        let topo_filename =
            async_compat::Compat::new(async { get_arcgis_topo_tile(x, y, z).await })
                .await
                .unwrap_or("".to_string());

        let image_filename =
            async_compat::Compat::new(async { get_arcgis_image_tile(x, y, z).await })
                .await
                .unwrap_or("".to_string());

        (topo_filename, image_filename)
    });
    commands.spawn().insert(task);
}

fn handle_tasks(
    mut commands: Commands,
    mut query_tasks: Query<(Entity, &mut Task<(String, String)>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    for (entity, mut task) in query_tasks.iter_mut() {
        if let Some((topo_filename, image_filename)) =
            future::block_on(future::poll_once(&mut *task))
        {
            setup_terrain(
                &mut commands,
                &mut meshes,
                &mut materials,
                &asset_server,
                &image_filename[7..],
                topo_filename.as_str(),
            );

            commands.entity(entity).remove::<Task<(String, String)>>();
        }
    }
}

fn setup(mut commands: Commands) {
    let mut camera_transform = Transform::from_translation(Vec3::new(-50., 50., -50.));
    camera_transform.look_at(Vec3::new(0., 0., 0.), Vec3::Y);

    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: camera_transform,
            perspective_projection: PerspectiveProjection {
                far: 2000.,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(OrbitCamera {
            x: 45.,
            y: 30.,
            zoom: 100.
        });

    commands.spawn_bundle(LightBundle {
        transform: Transform::from_translation(Vec3::new(0., 50., 0.)),
        light: Light {
            intensity: 10000.,
            range: 5000.,
            ..Default::default()
        },
        ..Default::default()
    });
}
